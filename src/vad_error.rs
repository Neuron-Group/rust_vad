use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum ErrorType {
    TpConfigErr,
    TpParseErr,
    TpTypeConvertErr,
    TpModelHandelErr,
    TpVoidErr,
}

pub trait VadError {
    fn get_error_type(&self) -> ErrorType {
        ErrorType::TpVoidErr
    }
}

#[derive(Debug, Clone)]
pub struct ConfigErr {
    err_type: ErrorType,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub struct ParseErr {
    err_type: ErrorType,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub struct TypeConvertErr {
    err_type: ErrorType,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub struct ModelHandelErr {
    err_type: ErrorType,
    pub msg: String,
}

// ConfigErr 实现
impl ConfigErr {
    pub fn new() -> Self {
        ConfigErr {
            err_type: ErrorType::TpConfigErr,
            msg: String::from("config failed >_<"),
        }
    }

    pub fn with_msg(msg: String) -> Self {
        ConfigErr {
            err_type: ErrorType::TpConfigErr,
            msg,
        }
    }
}

impl Default for ConfigErr {
    fn default() -> Self {
        Self::new()
    }
}

impl VadError for ConfigErr {
    fn get_error_type(&self) -> ErrorType {
        self.err_type.clone()
    }
}

impl fmt::Display for ConfigErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Config Error: {}", self.msg)
    }
}

impl Error for ConfigErr {}

// AlgorithmErr 实现
impl ParseErr {
    pub fn new() -> Self {
        Self {
            err_type: ErrorType::TpParseErr,
            msg: String::from("parse params failed >_<"),
        }
    }

    pub fn with_msg(msg: String) -> Self {
        Self {
            err_type: ErrorType::TpParseErr,
            msg,
        }
    }
}

impl Default for ParseErr {
    fn default() -> Self {
        Self::new()
    }
}

impl VadError for ParseErr {
    fn get_error_type(&self) -> ErrorType {
        self.err_type.clone()
    }
}

impl fmt::Display for ParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Parse Error: {}", self.msg)
    }
}

impl Error for ParseErr {}

// TypeConvertErr 实现
impl TypeConvertErr {
    pub fn new() -> Self {
        TypeConvertErr {
            err_type: ErrorType::TpTypeConvertErr,
            msg: String::from("type conversion failed >_<"),
        }
    }

    pub fn with_msg(msg: String) -> Self {
        TypeConvertErr {
            err_type: ErrorType::TpTypeConvertErr,
            msg,
        }
    }
}

impl Default for TypeConvertErr {
    fn default() -> Self {
        Self::new()
    }
}

impl VadError for TypeConvertErr {
    fn get_error_type(&self) -> ErrorType {
        self.err_type.clone()
    }
}

impl fmt::Display for TypeConvertErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Type Conversion Error: {}", self.msg)
    }
}

impl Error for TypeConvertErr {}

// ModelHandelErr 实现
impl ModelHandelErr {
    pub fn new() -> Self {
        ModelHandelErr {
            err_type: ErrorType::TpModelHandelErr,
            msg: String::from("model handling failed >_<"),
        }
    }

    pub fn with_msg(msg: String) -> Self {
        ModelHandelErr {
            err_type: ErrorType::TpModelHandelErr,
            msg,
        }
    }
}

impl Default for ModelHandelErr {
    fn default() -> Self {
        Self::new()
    }
}

impl VadError for ModelHandelErr {
    fn get_error_type(&self) -> ErrorType {
        self.err_type.clone()
    }
}

impl fmt::Display for ModelHandelErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Model Handling Error: {}", self.msg)
    }
}

impl Error for ModelHandelErr {}
