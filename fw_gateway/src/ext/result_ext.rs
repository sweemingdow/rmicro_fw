use crate::PingoraResult;

pub fn as_pingora_result<T, E, F>(fr: Result<T, E>, f: F) -> PingoraResult<T>
where
    E: ToString,
    F: FnOnce(&E) -> (pingora::ErrorType, pingora::ErrorSource),
{
    match fr {
        Ok(value) => Ok(value),
        Err(e) => {
            let (err_type, err_src) = f(&e);

            Err(pingora::Error::create(
                err_type,
                err_src,
                Some(e.to_string().into()),
                None,
            ))
        }
    }
}
