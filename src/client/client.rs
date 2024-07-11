mod cli;
mod log;
use {
    cli::build_cli, dusa_common::{
        get_id, prefix::{receive_message, send_message}, set_file_ownership, DecryptResponseData, Message, MessageType, RequestPayload, RequestRecsPlainText, RequestRecsSimple, RequestRecsWrite, SOCKET_PATH, VERSION
    }, nix::unistd::{geteuid}, pretty::{halt, pass, warn}, std::{
        fs, os::unix::net::UnixStream, path::PathBuf, process::exit, time::Duration
    }, system::{
        errors::{
            ErrorArray, ErrorArrayItem, OkWarning, UnifiedResult as uf, WarningArray,
            WarningArrayItem,
        }, types::PathType
    }
};

type Callback =
    fn(clap::ArgMatches, UnixStream, WarningArray, ErrorArray) -> uf<OkWarning<Option<String>>>;

fn main() {
    let e1: ErrorArray = ErrorArray::new_container();
    let w1: WarningArray = WarningArray::new_container();

    let socket_path: PathType = match SOCKET_PATH(false, e1.clone(), w1.clone()).uf_unwrap() {
        Ok(d) => {
            d.warning.display();
            d.data
        }
        Err(e) => {
            e.display(true);
            unreachable!()
        }
    };

    // clapping
    let cmd: clap::ArgMatches = build_cli().get_matches();

    enum ProgramMode {
        StoreFile(Callback),
        RetrieveFile(Callback),
        EncryptText(Callback),
        DecryptText(Callback),
        RemoveFile(Callback),
        Invalid,
    }

    // Get operating mode
    let (ef, df, et, dt, rf) = (
        cmd.get_flag("encrypt_file"),
        cmd.get_flag("decrypt_file"),
        cmd.get_flag("encrypt_text"),
        cmd.get_flag("decrypt_text"),
        cmd.get_flag("remove_file"),
    );

    let mode: ProgramMode = match (ef, df, et, dt, rf) {
        (true, false, false, false, false) => ProgramMode::StoreFile(encrypt_file),
        (false, true, false, false, false) => ProgramMode::RetrieveFile(decrypt_file),
        (false, false, true, false, false) => ProgramMode::EncryptText(encrypt_text),
        (false, false, false, true, false) => ProgramMode::DecryptText(decrypt_text),
        (false, false, false, false, true) => ProgramMode::RemoveFile(remove_file),
        _ => ProgramMode::Invalid,
    };

    let stream: UnixStream = match UnixStream::connect(socket_path.clone()) {
        Ok(d) => d,
        Err(_) => {
            halt("The server is not running or You do not have access to this application");
            unreachable!()
        },
    };

    let result: uf<OkWarning<Option<String>>> = match mode {
        ProgramMode::StoreFile(callback) => callback(cmd, stream, w1.clone(), e1.clone()),
        ProgramMode::RetrieveFile(callback) => callback(cmd, stream, w1.clone(), e1.clone()),
        ProgramMode::EncryptText(callback) => callback(cmd, stream, w1.clone(), e1.clone()),
        ProgramMode::DecryptText(callback) => callback(cmd, stream, w1.clone(), e1.clone()),
        ProgramMode::RemoveFile(callback) => callback(cmd, stream, w1.clone(), e1.clone()),
        ProgramMode::Invalid => {
            warn("Invalid command given use '-h' or '--help' for more info");
            exit(1)
        }
    };

    match result.uf_unwrap() {
        Ok(d) => d.warning.display(),
        Err(e) => e.display(false),
    }

    fn encrypt_file(
        cmd: clap::ArgMatches,
        mut stream: UnixStream,
        mut warnings: WarningArray,
        errors: ErrorArray,
    ) -> uf<OkWarning<Option<String>>> {
        // ensuring path exists
        let file_path: PathType = match get_file_path(
            errors.clone(),
            warnings.clone(),
            cmd.get_one::<PathBuf>("path"),
        )
        .uf_unwrap()
        {
            Ok(d) => {
                d.warning.display();
                d.data
            }
            Err(e) => return uf::new(Err(e)),
        };

        // Changing owner ship of the file
        let (uid, gid) = get_id();
        if let Err(err) = set_file_ownership(&file_path.to_path_buf(), uid, gid, errors.clone()).uf_unwrap() {
            return uf::new(Err(err))
        }

        // Creating the command to send
        let request_data = RequestRecsWrite {
            path: file_path,
            owner: cmd
                .get_one::<String>("owner")
                .unwrap_or(&String::from("system"))
                .to_owned(),
            name: cmd
                .get_one::<String>("name")
                .unwrap_or(&String::from("lost"))
                .to_string(),
            uid: u32::from(geteuid()),
        };

        let msg = Message {
            version: VERSION.to_owned(),
            msg_type: MessageType::Request,
            payload: serde_json::to_value(RequestPayload::Write(request_data)).unwrap(),
            error: None,
        };

        // Communicating with server
        if let Err(err) = send_message(&mut stream, &msg, errors.clone()).uf_unwrap() {
            return uf::new(Err(err))
        }
        std::thread::sleep(Duration::from_nanos(100));
        let response = receive_message(&mut stream, errors.clone()).unwrap();

        match response.msg_type {
            MessageType::Response => {
                let response_data = response.payload;
                let msg = response_data
                    .get("Ok")
                    .and_then(|v| v.as_str().map(|s| s.to_string()));

                // ping the file from the server 

                // delete the original file

                pass(&format!("{}", msg.unwrap()));
            }
            MessageType::ErrorResponse => {
                halt(&format!("We received the following error: {}", response.payload));
            }
            _ => {
                let msg = String::from("Server responded in an unexpected way, ignoring ...");
                warnings.push(WarningArrayItem::new_details(
                    system::errors::Warnings::Warning,
                    msg,
                ))
            }
        }

        uf::new(Ok(OkWarning {
            data: None,
            warning: warnings,
        }))
    }

    fn decrypt_file(
        cmd: clap::ArgMatches,
        mut stream: UnixStream,
        mut warnings: WarningArray,
        mut errors: ErrorArray,
    ) -> uf<OkWarning<Option<String>>> {
        let request_data = RequestRecsSimple {
            command: dusa_common::Commands::DecryptFile,
            owner: cmd
                .get_one::<String>("owner")
                .unwrap_or(&String::from("system"))
                .to_owned(),
            name: cmd
                .get_one::<String>("name")
                .unwrap_or(&String::from("lost"))
                .to_string(),
            uid: u32::from(geteuid()),
        };

        println!("{:?}", &request_data);

        let msg = Message {
            version: VERSION.to_owned(),
            msg_type: MessageType::Request,
            payload: serde_json::to_value(RequestPayload::Simple(request_data)).unwrap(),
            error: None,
        };

        // Communicating with server
        if let Err(err) =
        send_message(&mut stream, &msg, errors.clone())
            .uf_unwrap()
    {
        err.display(false)
    }
        // std::thread::sleep(Duration::from_nanos(100));
        let response = receive_message(&mut stream, errors.clone()).unwrap();

        match response.msg_type {
            MessageType::Response => {
                let response_data = response.payload;
                let data = DecryptResponseData {
                    temp_p: response_data
                        .get("temp_p")
                        .and_then(|v| v.get("Content"))
                        .and_then(|v| v.as_str())
                        .map(|s| PathType::Content(s.to_string()))
                        .unwrap_or_else(|| PathType::Content("/tmp/null".to_string())),                
                    orig_p: response_data
                        .get("orig_p")
                        .and_then(|v| v.get("PathBuf"))
                        .and_then(|v| v.as_str())
                        .map(|s| PathType::Content(s.to_string()))
                        .unwrap_or_else(|| PathType::Content("/tmp/null".to_string())),  
                    ttl: response_data
                        .get("ttl")
                        .and_then(|v| v.get("secs"))
                        .and_then(|v| v.as_u64())
                        .map(|t| Duration::from_secs(t))
                        .unwrap_or(Duration::from_secs(5)), // keep the timing tight
                };
                let data_cloned = data.clone();

                // Send an ACK message
                let ack = Message {
                    version: VERSION.to_owned(),
                    msg_type: MessageType::Acknowledge,
                    payload: serde_json::json!({}),
                    error: None,
                };
                send_message(&mut stream, &ack, errors.clone());
                let _ = receive_message(&mut stream, errors.clone()).unwrap();

                // copy the file to the original path
                match fs::copy(data.temp_p, data.orig_p) {
                    Ok(d) => if d != 0 {
                        pass(&format!("{:#?}", data_cloned));
                    },
                    Err(e) => {
                        errors.push(ErrorArrayItem::from(e));
                        errors.display(true);
                        unreachable!()
                    },
                }
            }
            MessageType::ErrorResponse => {
                halt(&format!("{}", response.payload));
            }
            _ => {
                let msg = String::from("Server responded in an unexpected way, ignoring ...");
                warnings.push(WarningArrayItem::new_details(
                    system::errors::Warnings::Warning,
                    msg,
                ))
            }
        }

        return uf::new(Ok(OkWarning {
            data: None,
            warning: warnings,
        }));
    }

    fn encrypt_text(
        cmd: clap::ArgMatches,
        mut stream: UnixStream,
        mut warnings: WarningArray,
        errors: ErrorArray,
    ) -> uf<OkWarning<Option<String>>> {
        let request_data = RequestRecsPlainText {
            command: dusa_common::Commands::EncryptRawText,
            data: cmd
                .get_one::<String>("data")
                .unwrap_or(&String::from("hello world"))
                .to_owned(),
            uid: unsafe { u32::from(geteuid()) },
        };

        let msg = Message {
            version: VERSION.to_owned(),
            msg_type: MessageType::Request,
            payload: serde_json::to_value(RequestPayload::PlainText(request_data)).unwrap(),
            error: None,
        };

        // Communicating with server
        let _ = send_message(&mut stream, &msg, errors.clone());
        std::thread::sleep(Duration::from_nanos(100));
        let response = receive_message(&mut stream, errors.clone()).unwrap();

        match response.msg_type {
            MessageType::Response => {
                let response_data = response.payload;

                // Send an ACK message
                let ack = Message {
                    version: VERSION.to_owned(),
                    msg_type: MessageType::Acknowledge,
                    payload: serde_json::json!({}),
                    error: None,
                };
                send_message(&mut stream, &ack, errors.clone());
                let _ = receive_message(&mut stream, errors.clone()).unwrap();

                pass(
                    response_data
                        .get("value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Invalid data received"),
                );
            }
            MessageType::ErrorResponse => {
                halt(&format!("{}", response.payload));
            }
            _ => {
                let msg = String::from("Server responded in an unexpected way, ignoring ...");
                warnings.push(WarningArrayItem::new_details(
                    system::errors::Warnings::Warning,
                    msg,
                ))
            }
        };

        return uf::new(Ok(OkWarning {
            warning: warnings,
            data: None,
        }));
    }

    fn decrypt_text(
        cmd: clap::ArgMatches,
        mut stream: UnixStream,
        mut warnings: WarningArray,
        errors: ErrorArray,
    ) -> uf<OkWarning<Option<String>>> {
        let request_data = RequestRecsPlainText {
            command: dusa_common::Commands::DecryptRawText,
            data: cmd
                .get_one::<String>("data")
                .unwrap_or(&String::from("hello world"))
                .to_owned(),
            uid: unsafe { u32::from(geteuid()) },
        };

        let msg = Message {
            version: VERSION.to_owned(),
            msg_type: MessageType::Request,
            payload: serde_json::to_value(RequestPayload::PlainText(request_data)).unwrap(),
            error: None,
        };

        // Communicating with server
        let _ = send_message(&mut stream, &msg, errors.clone());
        std::thread::sleep(Duration::from_nanos(100));
        let response = receive_message(&mut stream, errors.clone()).unwrap();

        match response.msg_type {
            MessageType::Response => {
                let response_data = response.payload;

                // Send an ACK message
                let ack = Message {
                    version: VERSION.to_owned(),
                    msg_type: MessageType::Acknowledge,
                    payload: serde_json::json!({}),
                    error: None,
                };
                send_message(&mut stream, &ack, errors.clone());
                let _ = receive_message(&mut stream, errors.clone()).unwrap();

                pass(
                    response_data
                        .get("value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Invalid data received"),
                );
            }
            MessageType::ErrorResponse => {
                halt(&format!("{}", response.payload));
            }
            _ => {
                let msg = String::from("Server responded in an unexpected way, ignoring ...");
                warnings.push(WarningArrayItem::new_details(
                    system::errors::Warnings::Warning,
                    msg,
                ))
            }
        };

        return uf::new(Ok(OkWarning {
            warning: warnings,
            data: None,
        }));
    }

    fn remove_file(
        cmd: clap::ArgMatches,
        mut stream: UnixStream,
        mut warnings: WarningArray,
        errors: ErrorArray,
    ) -> uf<OkWarning<Option<String>>> {
        let request_data = RequestRecsSimple {
            command: dusa_common::Commands::RemoveFile,
            owner: cmd
                .get_one::<String>("owner")
                .unwrap_or(&String::from("system"))
                .to_owned(),
            name: cmd
                .get_one::<String>("name")
                .unwrap_or(&String::from("lost"))
                .to_string(),
            uid: unsafe { u32::from(geteuid()) },
        };

        let msg = Message {
            version: VERSION.to_owned(),
            msg_type: MessageType::Request,
            payload: serde_json::to_value(RequestPayload::Simple(request_data)).unwrap(),
            error: None,
        };

        // Communicating with server
        let _ = send_message(&mut stream, &msg, errors.clone());
        std::thread::sleep(Duration::from_nanos(100));
        let response = receive_message(&mut stream, errors.clone()).unwrap();

        match response.msg_type {
            MessageType::Response => {
                let response_data = response.payload;

                // Send an ACK message
                let ack = Message {
                    version: VERSION.to_owned(),
                    msg_type: MessageType::Acknowledge,
                    payload: serde_json::json!({}),
                    error: None,
                };
                send_message(&mut stream, &ack, errors.clone());
                let _ = receive_message(&mut stream, errors.clone()).unwrap();

                pass(
                    response_data
                        .get("value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Invalid data recived"),
                );
            }
            MessageType::ErrorResponse => {
                halt(&format!("{}", response.payload));
            }
            _ => {
                let msg = String::from("Server responded in an unexpected way, ignoring ...");
                warnings.push(WarningArrayItem::new_details(
                    system::errors::Warnings::Warning,
                    msg,
                ))
            }
        };

        return uf::new(Ok(OkWarning {
            warning: warnings,
            data: None,
        }));
    }

    fn get_file_path(
        mut errors: ErrorArray,
        _warnings: WarningArray,
        option_path_ref: Option<&PathBuf>,
    ) -> uf<OkWarning<PathType>> {
        let err = match option_path_ref {
            Some(d) => match d.to_path_buf().canonicalize() {
                Ok(d) => {
                    let result = OkWarning {
                        data: PathType::PathBuf(d),
                        warning: _warnings,
                    };
                    return uf::new(Ok(result));
                }
                Err(err) => ErrorArrayItem::from(err),
            },
            None => ErrorArrayItem::new(system::errors::Errors::InvalidFile, "".to_owned()),
        };
        errors.push(err);
        return uf::new(Err(errors));
    }

    // let request_data = RequestRecsWrite {
    //     command: "write".to_string(),
    //     path: "/example/path".to_string(),
    //     owner: "user".to_string(),
    //     name: "example".to_string(),
    //     uid: 1,
    // };

    // msg_vec.push(Message {
    //     version: VERSION.to_owned(),
    //     msg_type: MessageType::Request,
    //     payload: serde_json::to_value(RequestPayload::Write(request_data)).unwrap(),
    //     error: None,
    // });

    // msg_vec.push(Message {
    //     version: VERSION.to_owned(),
    //     msg_type: MessageType::Simple,
    //     payload: json!({"Key": "Hello world"}),
    //     error: None,
    // });

    // let request_data = RequestRecsPlainText {
    // command: "Write raw".to_string(),
    // data: "32333666336336313233313439643065303464356538353963356139316637303d33303331326435323331326533303265333232643335333036363336333336343634333933323334333736313632333436323330333436313632363232643331613361323366313237333935633462366537663832343266343838663761353364653235373364353864613839633964356136396234623234643939333530666745744162617432317769333334546f646533663735333761646536643136633562376631376264363037316464646430343639393937333332303861343764653739363637323230383435353434653d31".to_string(),
    // uid: 1,
    // };

    // let request_data = RequestRecsWrite {
    //     path: PathType::Content("/tmp/dusa/general.log".to_string()),
    //     owner: String::from("dwhitfield"),
    //     name: String::from("test"),
    //     uid: 1000,
    // };

    // let request_data = RequestRecsSimple {

    // }

    // let request_data = RequestRecsPlainText {
    //     uid: 1000,
    //     command: dusa_common::Commands::EncryptRawText,
    //     data: "hello world".to_string(),
    // };

    // let msg = Message {
    //     version: VERSION.to_owned(),
    //     msg_type: MessageType::Request,
    //     payload: serde_json::to_value(RequestPayload::Write(request_data)).unwrap(),
    //     error: None,
    // };

    // let _ = send_message(&mut stream, &msg, e1.clone());

    // // print!("{:#?}", receive_message(&mut stream, e1.clone()));
    // std::thread::sleep(Duration::from_nanos(100));
    // let response = receive_message(&mut stream, e1.clone()).unwrap();

    // match response.msg_type {
    // MessageType::Response => todo!(),
    // MessageType::ErrorResponse => todo!(),
    // MessageType::Simple => todo!(),
    // MessageType::Acknowledge => todo!(),
    // }
    // print!("{:#?}", response);

    // if let Some(error) = response.error {
    //     match error.code {
    //         ErrorCode::UnknownMessageType => {
    //             println!("Error: Unknown message type");
    //         }
    //         ErrorCode::InvalidPayload => {
    //             println!("Error: Invalid payload");
    //         }
    //         ErrorCode::InternalError => {
    //             println!("Error: Internal error");
    //         }
    //         ErrorCode::InvalidVersion => {
    //             println!("Error: We are'nt speaking the same language")
    //         }
    //         ErrorCode::InvalidPermissions => {
    //             println!("The bouncer said our id was fake")
    //         } // Handle more error codes as needed
    //     }
    //     println!("Error message: {}", error.message);
    // } else {
    //     if response.msg_type == MessageType::Acknowledge {
    //         println!("Received ACK from server, closing connection.");
    //         // return to whatevet
    //         // std::process::exit(0)
    //     } else {
    //         println!("Received response: {:?}", response);
    //     }
    // }

    // // Send an ACK message
    // let ack = Message {
    //     version: VERSION.to_owned(),
    //     msg_type: MessageType::Acknowledge,
    //     payload: serde_json::json!({}),
    //     error: None,
    // };
    // let ack_data = serde_json::to_vec(&ack).unwrap();
    // stream.write_all(&ack_data).unwrap();

    // // Read the server's ACK message before closing
    // let n = stream.read(&mut buffer).unwrap();
    // let server_ack: Message<serde_json::Value> = serde_json::from_slice(&buffer[..n]).unwrap();
    // if server_ack.msg_type == MessageType::Acknowledge {
    //     println!("Received ACK from server, closing connection.");
    // }

}
