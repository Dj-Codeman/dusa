use logging::append_log;
use system::errors::ErrorArray;
use users::get_current_uid;


pub fn log(data: String) {
    let progname = format!("dusa-cli-{}", get_current_uid());
    let errors: ErrorArray = ErrorArray::new_container();

    if let Err(e) = append_log(&progname, &data, errors.clone()).uf_unwrap() {
        e.display(false);
    }

    drop(errors);
    drop(data);
    drop(progname);
}
