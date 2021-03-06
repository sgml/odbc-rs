use super::{ffi, safe};
use std::fmt;
use std::ffi::CStr;
use std::error::Error;

/// ODBC Diagnostic Record
///
/// The `description` method of the `std::error::Error` trait only returns the message. Use
/// `std::fmt::Display` to retrive status code and other information.
pub struct DiagnosticRecord {
    // All elements but the last one, may not be nul. The last one must be nul.
    state: [ffi::SQLCHAR; ffi::SQL_SQLSTATE_SIZE + 1],
    // Must at least contain one nul
    message: [ffi::SQLCHAR; ffi::SQL_MAX_MESSAGE_LENGTH as usize],
    // The numbers of characters in message not nul
    message_length: ffi::SQLSMALLINT,
    native_error: ffi::SQLINTEGER,
}

impl DiagnosticRecord {
    /// get raw state string data.
    pub fn get_raw_state(&self) -> &[u8] {
        &self.state
    }
    /// get raw diagnostics message for avoiding encoding error.
    pub fn get_raw_message(&self) -> &[u8] {
        &self.message[0..self.message_length as usize]
    }
    /// get native odbc error number
    pub fn get_native_error(&self) -> i32 {
        self.native_error
    }
}

impl fmt::Display for DiagnosticRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Todo: replace unwrap with `?` in Rust 1.17
        let state = CStr::from_bytes_with_nul(&self.state).unwrap();
        let message = CStr::from_bytes_with_nul(
            &self.message[0..(self.message_length as usize + 1)],
        ).unwrap();

        write!(
            f,
            "State: {}, Native error: {}, Message: {}",
            state.to_str().unwrap(),
            self.native_error,
            message.to_str().unwrap()
        )
    }
}

impl fmt::Debug for DiagnosticRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for DiagnosticRecord {
    fn description(&self) -> &str {
        CStr::from_bytes_with_nul(&self.message[0..(self.message_length as usize + 1)])
            .unwrap()
            .to_str()
            .unwrap()
    }
    fn cause(&self) -> Option<&Error> {
        None
    }
}

/// Allows retriving a diagnostic record, describing errors (or lack thereof) during the last
/// operation.
pub trait GetDiagRec {
    /// Retrieves a diagnostic record
    ///
    /// `record_number` - Record numbers start at one. If you pass an number < 1 the function will
    /// panic. If no record is available for the number specified none is returned.
    fn get_diag_rec(&self, record_number: i16) -> Option<DiagnosticRecord>;
}

impl<D> GetDiagRec for D
where
    D: safe::Diagnostics,
{
    fn get_diag_rec(&self, record_number: i16) -> Option<(DiagnosticRecord)> {
        use safe::ReturnOption::*;
        let mut message = [0; 512];
        match self.diagnostics(record_number, &mut message) {
            Success(result) | Info(result) => {
                Some(DiagnosticRecord {
                    state: result.state,
                    native_error: result.native_error,
                    message_length: result.text_length,
                    message,
                })
            }
            NoData(()) => None,
            Error(()) => panic!("Diagnostics returned error for record number {}. Record numbers have to be at least 1.", record_number),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    impl DiagnosticRecord {
        fn new() -> DiagnosticRecord {
            DiagnosticRecord {
                state: [0u8; ffi::SQL_SQLSTATE_SIZE + 1],
                message: [0u8; ffi::SQL_MAX_MESSAGE_LENGTH as usize],
                native_error: 0,
                message_length: 0,
            }
        }
    }

    #[test]
    fn formatting() {

        // build diagnostic record
        let message = b"[Microsoft][ODBC Driver Manager] Function sequence error\0";
        let mut rec = DiagnosticRecord::new();
        rec.state = b"HY010\0".clone();
        rec.message_length = 56;
        for i in 0..(rec.message_length as usize) {
            rec.message[i] = message[i];
        }

        // test formatting
        assert_eq!(
            format!("{}", rec),
            "State: HY010, Native error: 0, Message: [Microsoft][ODBC Driver Manager] \
             Function sequence error"
        );
    }
}
