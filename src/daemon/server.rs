pub mod response_err;

use dusa_common::{
    check_version, get_id, prefix::{receive_message, send_message, GeneralMessage}, set_file_ownership, set_socket_permission, DecryptResponseData, DusaError, ErrorCode, Message, MessageType, RequestPayload, RequestRecsSimple, SOCKET_PATH, TTL, VERSION
};
use nix::unistd::{setgid, setuid};
use simple_pretty::{halt, notice, output};
use recs::{decrypt_raw, encrypt_raw, initialize, remove, retrieve, store};
use serde_json::json;
use std::{
    os::unix::net::{UnixListener, UnixStream}, thread::{self}, time::Duration
};
use dusa_collection_utils::{
    errors::{ErrorArray, ErrorArrayItem, Errors, WarningArray}, functions::del_file, types::{ClonePath, PathType}
};

fn main() {
    // Initializing 1st errors and warnings
    let e1: ErrorArray = ErrorArray::new_container();
    let w1: WarningArray = WarningArray::new_container();

    // Make sure we are running as the dusa user
    let (uid, gid) = get_id();
    match (setuid(uid.into()), setgid(gid.into())) {
        (Ok(_), Ok(_)) => (),
        _ => halt("We aren't running as the correct user, peacing out .."),
    };

    pub const PROGNAME: &str = "dusa";
    // Initializing recs
    recs::set_debug(false);
    recs::set_prog(PROGNAME);

    if let Err(mut err) = initialize(e1.clone(), w1.clone()).uf_unwrap() {
        err.push(ErrorArrayItem::new(
            Errors::GeneralError,
            "Recs failed to initialize".to_string(),
        ));
        err.display(true);
    }

    let socket_path: PathType = match SOCKET_PATH(true, e1.clone(), w1.clone()).uf_unwrap() {
        Ok(d) => {
            d.warning.display();
            d.data
        }
        Err(e) => {
            e.display(true);
            unreachable!()
        }
    };

    // Setting up the new socket file
    let listener: UnixListener = match UnixListener::bind(socket_path.clone_path()) {
        Ok(d) => d,
        Err(e) => {
            halt(&format!(
                "We couldn't create the socket because this happened: {}",
                &e.to_string()
            ));
            unreachable!()
        }
    };

    // setting correct permissions on the socket
    set_socket_permission(socket_path.clone()); // return an error

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let e2: ErrorArray = ErrorArray::new_container();
                let w2: WarningArray = WarningArray::new_container();
                // Spawn a new thread or use async/await to handle each incoming connection
                thread::spawn(move || handle_client(stream, e2.clone(), w2.clone()));
            }
            Err(e) => halt(&format!("Error accepting connection: {}", e)),
        }
    }
}

#[allow(unreachable_patterns)]
fn handle_client(mut stream: UnixStream, errors: ErrorArray, warnings: WarningArray) {
    let new_message: GeneralMessage = match receive_message(&mut stream, errors.clone()).uf_unwrap()
    {
        Ok(d) => d,
        Err(e) => {
            e.display(false);
            return
        }
    };

    // Checking the message version
    if !check_version(&new_message.version) {
        let error = DusaError {
            code: ErrorCode::InvalidVersion,
            message: format!(
                "Client and Server out of date. Server version: {}, Client version: {}",
                VERSION, &new_message.version
            ),
        };
        let response = Message {
            version: VERSION.to_owned(),
            msg_type: MessageType::ErrorResponse,
            payload: serde_json::json!({}),
            error: Some(error),
        };
        if let Err(err) = send_message(&mut stream, &response, errors.clone()).uf_unwrap() {
            err.display(false)
        }
    }

    match new_message.msg_type {
        MessageType::Request => {
            // Deserialize the payload into a specific struct
            let request_payload: RequestPayload = serde_json::from_value(new_message.payload)
                .unwrap_or(RequestPayload::Simple(RequestRecsSimple {
                    command: dusa_common::Commands::PingFile,
                    owner: "system".to_string(),
                    name: "test".to_string(),
                    uid: 1000,
                }));

            let request_data: RequestPayload = match request_payload {
                RequestPayload::Write(d) => RequestPayload::Write(d),
                RequestPayload::PlainText(d) => RequestPayload::PlainText(d),
                RequestPayload::Simple(d) => RequestPayload::Simple(d),
            };

            match request_data {
                RequestPayload::Write(req) => {
                    let owner = req.owner;
                    let name = req.name;
                    let path = req.path;
                    let _uid = req.uid;
                    match store(
                        path.clone_path(),
                        owner,
                        name,
                        errors.clone(),
                        warnings.clone(),
                    )
                    .uf_unwrap()
                    {
                        Ok(_) => {
                            let response = Message {
                                version: VERSION.to_owned(),
                                msg_type: MessageType::Response,
                                payload: serde_json::json!({"Ok":format!("file {} written", path)}),
                                error: None,
                            };
                            if let Err(err) =
                                send_message(&mut stream, &response, errors.clone()).uf_unwrap()
                            {
                                err.display(false)
                            }
                            output("GREEN", "done");
                        }
                        Err(e) => {
                            e.display(false);
                            let response = Message {
                                version: VERSION.to_owned(),
                                msg_type: MessageType::ErrorResponse,
                                payload: serde_json::json!({"Error":"Error occurred while inserting"}),
                                error: None,
                            };
                            if let Err(err) =
                                send_message(&mut stream, &response, errors.clone()).uf_unwrap()
                            {
                                err.display(false)
                            }
                        }
                    }

                    // Send an ACK message
                    send_message(
                        &mut stream,
                        &Message {
                            version: VERSION.to_owned(),
                            msg_type: MessageType::Acknowledge,
                            payload: serde_json::json!({}),
                            error: None,
                        },
                        errors.clone(),
                    );

                    return
                }
                RequestPayload::PlainText(req) => {
                    let command = req.command;
                    let data = req.data;
                    let _uid = req.uid;

                    match command {
                        dusa_common::Commands::EncryptRawText => {
                            match encrypt_raw(data, errors.clone(), warnings.clone()).uf_unwrap() {
                                Ok((key, cipher, chunks)) => {
                                    let data: String = format!("{}-{}-{}", cipher, key, chunks);
                                    let response = Message {
                                        version: VERSION.to_string(),
                                        msg_type: MessageType::Response,
                                        payload: json!({"value":data}),
                                        error: None,
                                    };
                                    if let Err(err) =
                                        send_message(&mut stream, &response, errors.clone())
                                            .uf_unwrap()
                                    {
                                        err.display(false)
                                    }
                                }
                                Err(e) => {
                                    e.display(false);
                                    let response = Message {
                                        version: VERSION.to_owned(),
                                        msg_type: MessageType::ErrorResponse,
                                        payload: serde_json::json!({"Error":"Error occurred while encrypting the data"}),
                                        error: None,
                                    };
                                    if let Err(err) =
                                        send_message(&mut stream, &response, errors.clone())
                                            .uf_unwrap()
                                    {
                                        err.display(false)
                                    };
                                }
                            }
                        }
                        dusa_common::Commands::DecryptRawText => {
                            let parts: Vec<&str> = data.split('-').collect();
                            let recs_data = parts.get(0).unwrap_or(&"").to_string();
                            let recs_key = parts.get(1).unwrap_or(&"").to_string();
                            let recs_chunks =
                                parts.get(2).unwrap_or(&"1").parse::<usize>().unwrap_or(1);

                            match decrypt_raw(
                                recs_data,
                                recs_key,
                                recs_chunks,
                                errors.clone(),
                                warnings,
                            )
                            .uf_unwrap()
                            {
                                Ok(d) => {
                                    let data = d.data.clone();
                                    let message = String::from_utf8(data).unwrap();
                                    d.warning.display();
                                    let response = Message {
                                        version: VERSION.to_owned(),
                                        msg_type: MessageType::Response,
                                        payload: serde_json::json!({"value":message}),
                                        error: None,
                                    };
                                    if let Err(err) =
                                        send_message(&mut stream, &response, errors.clone())
                                            .uf_unwrap()
                                    {
                                        err.display(false)
                                    };
                                }
                                Err(e) => {
                                    let response = Message {
                                        version: VERSION.to_owned(),
                                        msg_type: MessageType::ErrorResponse,
                                        payload: serde_json::json!({"Error":"Error occurred while decrypting the data"}),
                                        error: None,
                                    };
                                    if let Err(err) =
                                        send_message(&mut stream, &response, errors.clone())
                                            .uf_unwrap()
                                    {
                                        err.display(false)
                                    } else {
                                        e.display(false)
                                    }
                                }
                            }
                        }
                        _ => {
                            let error = DusaError {
                                code: ErrorCode::InternalError,
                                message: "Invalid command parsing".to_string(),
                            };
                            let response = Message {
                                version: VERSION.to_owned(),
                                msg_type: MessageType::ErrorResponse,
                                payload: serde_json::json!({}),
                                error: Some(error),
                            };
                            if let Err(err) =
                                send_message(&mut stream, &response, errors.clone()).uf_unwrap()
                            {
                                err.display(false)
                            }
                        }
                    }
                }
                RequestPayload::Simple(req) => {
                    let owner = req.owner;
                    let name = req.name;
                    let _uid = req.uid;

                    match req.command {
                        dusa_common::Commands::DecryptFile => {
                            match retrieve(owner, name, _uid, errors.clone(), warnings.clone())
                                .uf_unwrap()
                            {
                                Ok(d) => {
                                    let data = d.data;
                                    let temp_p = data.0.clone();
                                    let temp_p_clone = data.0.clone();
                                    let orig_p = data.1;
                                    // The temp path will be deleted after the ttl time
                                    let ttl = Duration::from_secs(TTL);
                                    let decrypt_response = DecryptResponseData {
                                        temp_p,
                                        orig_p,
                                        ttl,
                                    };
                                    d.warning.display();

                                    let response = Message {
                                        version: VERSION.to_owned(),
                                        msg_type: MessageType::Response,
                                        payload: decrypt_response,
                                        error: None,
                                    };

                                    if let Err(err) =
                                        send_message(&mut stream, &response, errors.clone())
                                            .uf_unwrap()
                                    {
                                        err.display(false)
                                    }

                                    thread::spawn(move || {
                                        thread::sleep(Duration::from_secs(TTL));
                                        // taking back ownership
                                        let (uid, gid) = get_id();
                                        if let Err(err) = set_file_ownership(&temp_p_clone.to_path_buf(), uid, gid, ErrorArray::new_container()).uf_unwrap() {
                                            err.display(false);
                                            return
                                        }
                                        match del_file(
                                            temp_p_clone,
                                            ErrorArray::new_container(),
                                            WarningArray::new_container(),
                                        )
                                        .uf_unwrap()
                                        {
                                            Ok(_) => notice("Cleaning up temp files"),
                                            Err(e) => {
                                                e.display(false)
                                            },
                                        }
                                    });


                                }
                                Err(e) => {
                                    let response = Message {
                                        version: VERSION.to_owned(),
                                        msg_type: MessageType::ErrorResponse,
                                        payload: serde_json::json!({"Error":"Error occurred while decrypting the data"}),
                                        error: None,
                                    };
                                    if let Err(err) =
                                        send_message(&mut stream, &response, errors.clone())
                                            .uf_unwrap()
                                    {
                                        err.display(false)
                                    } else {
                                        e.display(false)
                                    }
                                }
                            }
                        }
                        dusa_common::Commands::RemoveFile => {
                            match remove(owner, name, errors.clone(), warnings.clone()).uf_unwrap()
                            {
                                Ok(_) => {
                                    let ack = Message {
                                        version: new_message.version,
                                        msg_type: MessageType::Response,
                                        payload: serde_json::json!({"value":"Ok"}),
                                        error: None,
                                    };
                                    if let Err(err) =
                                        send_message(&mut stream, &ack, errors.clone())
                                            .uf_unwrap()
                                    {
                                        err.display(false)
                                    }                                }
                                Err(e) => {
                                    let response = Message {
                                        version: VERSION.to_owned(),
                                        msg_type: MessageType::ErrorResponse,
                                        payload: serde_json::json!({"Error":"Error occurred while decrypting the data"}),
                                        error: None,
                                    };
                                    if let Err(err) =
                                        send_message(&mut stream, &response, errors.clone())
                                            .uf_unwrap()
                                    {
                                        err.display(false)
                                    } else {
                                        e.display(false)
                                    }
                                }
                            }
                        }
                        dusa_common::Commands::PingFile => {
                            let ack = Message {
                                version: VERSION.to_owned(),
                                msg_type: MessageType::Response,
                                payload: serde_json::json!({"value":"Not implemented"}),
                                error: None,
                            };
                            if let Err(err) =
                                send_message(&mut stream, &ack, errors.clone()).uf_unwrap()
                            {
                                err.display(false)
                            }
                        }
                        _ => {
                            let error = DusaError {
                                code: ErrorCode::InternalError,
                                message: "Invalid command parsing".to_string(),
                            };
                            let response = Message {
                                version: VERSION.to_owned(),
                                msg_type: MessageType::ErrorResponse,
                                payload: serde_json::json!({}),
                                error: Some(error),
                            };
                            if let Err(err) =
                                send_message(&mut stream, &response, errors.clone()).uf_unwrap()
                            {
                                err.display(false)
                            }
                        }
                    }
                }
            }

            // At the end of any transmission we expect an ack to be sent and received to ensure all data was captured

            // Send an ACK message
            send_message(
                &mut stream,
                &Message {
                    version: VERSION.to_owned(),
                    msg_type: MessageType::Acknowledge,
                    payload: serde_json::json!({}),
                    error: None,
                },
                errors.clone(),
            );
        }
        MessageType::Simple => {
            // Send an ACK message
            let ack = Message {
                version: new_message.version,
                msg_type: MessageType::Acknowledge,
                payload: serde_json::json!({}),
                error: None,
            };
            send_message(&mut stream, &ack, errors.clone());
        }
        _ => {
            // Unknown type
            let error = DusaError {
                code: ErrorCode::UnknownMessageType,
                message: "Unknown message type".to_string(),
            };
            let response = Message {
                version: VERSION.to_owned(),
                msg_type: MessageType::ErrorResponse,
                payload: serde_json::json!({}),
                error: Some(error),
            };
            send_message(&mut stream, &response, errors.clone());
        }
    }
}
