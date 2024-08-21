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
macro_rules! unwrap_or_else {
    ($res: expr, $else_block:block) => {
        match $res {
            Some(v) => v,
            None => $else_block,
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
macro_rules! read_lock_or_return {
    ($lock:expr, $ret_val:expr) => {
        match $lock.read() {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Warning: Failed to acquire read lock on RwLock, {}", e);
                return $ret_val;
            }
        }
    };
}

#[macro_export]
macro_rules! read_lock_or_else {
    ($lock:expr, $else_block:block) => {
        match $lock.read() {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Warning: Failed to acquire read lock on RwLock {}", e);
                $else_block
            }
        }
    };
}
