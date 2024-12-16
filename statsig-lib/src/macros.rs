#[macro_export]
macro_rules! unwrap_or_return {
    ($res: expr, $code: expr) => {
        match $res {
            Some(v) => v,
            None => return $code,
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_return_with {
    ($res: expr, $func:expr) => {
        match $res {
            Some(v) => v,
            None => return $func(),
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_else {
    ($res: expr, $else_block:block) => {
        match $res {
            Some(v) => v,
            None => $else_block,
        }
    };
}

#[macro_export]
macro_rules! ok_or_return_with {
    ($res:expr, $func:expr) => {
        match $res {
            Ok(v) => v,
            Err(e) => return $func(e),
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_noop {
    ($res: expr) => {
        match $res {
            Some(v) => v,
            None => return,
        }
    };
}

#[macro_export]
macro_rules! read_lock_or_else {
    ($lock:expr, $else_block:block) => {
        match $lock.read() {
            Ok(data) => data,
            Err(_) => $else_block,
        }
    };
}

#[macro_export]
macro_rules! read_lock_or_return {
    ($tag: expr, $lock:expr, $code: expr) => {
        match $lock.read() {
            Ok(data) => data,
            Err(e) => {
                log_e!($tag, "Failed to read store: {}", e.to_string());
                return $code;
            },
        }
    };
}


