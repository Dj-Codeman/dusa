use pretty::{output, warn};
use recs::errors::{RecsError, RecsWarning};

// Handeling warnings and errors
#[derive(Debug, Clone)]
pub struct Warnings(pub Vec<RecsWarning>);
#[derive(Debug, Clone)]
pub struct Errors(pub Vec<RecsError>);

#[derive(Debug, Clone)]
pub struct OkWarning<T> {
    pub data: T,
    pub warning: Warnings,
}

#[derive(Debug, Clone)]
pub struct UnifiedResult<T>(Result<OkWarning<T>, Errors>);

impl<T> UnifiedResult<T> {
    pub fn new(result: Result<OkWarning<T>, Errors>) -> Self {
        UnifiedResult(result)
    }

    pub fn resolve(self) -> T {
        match self.0 {
            Ok(o) => {
                o.warning.display();
                return o.data;
            }
            Err(e) => {
                e.display();
                std::process::exit(1);
            }
        }
    }

    pub fn unwrap(self) -> Result<T, Errors>{
        match self.0 {
            Ok(d) => return Ok(d.data),
            Err(e) => return Err(e),
        }
    }
}

impl Warnings {
    pub fn new(data: Vec<RecsWarning>) -> Self {
        Self { 0: data }
    }

    pub fn new_container() -> Self {
        return Self { 0: Vec::new() };
    }

    pub fn display(self) {
        for warns in self.0 {
            warn(&format!("{}", warns))
        }
    }

    pub fn push(mut self, item: RecsWarning) {
        self.0.push(item)
    }
}

impl Errors {
    pub fn new(data: Vec<RecsError>) -> Self {
        Self { 0: data }
    }

    pub fn new_container() -> Self {
        return Self { 0: Vec::new() };
    }

    pub fn display(self) {
        for errs in self.0 {
            output("RED", &format!("{}", errs))
        }
    }

    pub fn push(mut self, item: RecsError) {
        self.0.push(item)
    }
}
