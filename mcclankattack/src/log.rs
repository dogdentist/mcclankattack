#[macro_export]
macro_rules! outputln {
    () => {};
    ($($arg:tt)*) => {{
        use std::io::Write;

        let timestamp = chrono::Local::now().timestamp_millis();
        let timestamp = chrono::DateTime::from_timestamp_millis(timestamp).unwrap();
        let timestamp = timestamp.to_rfc3339();
        let message = format!($($arg)*);
        let log_line = timestamp + " LOG " + file!() + ":" + &line!().to_string() + " :: " + &message + "\n";
        let log_line = log_line.as_bytes();

        let _ = std::io::stdout().write(log_line);
    }};
}

#[macro_export]
macro_rules! errorln {
    () => {};
    ($($arg:tt)*) => {{
        use std::io::Write;

        let timestamp = chrono::Local::now().timestamp_millis();
        let timestamp = chrono::DateTime::from_timestamp_millis(timestamp).unwrap();
        let timestamp = timestamp.to_rfc3339();
        let message = format!($($arg)*);
        let log_line = timestamp + " ERR " + file!() + ":" + &line!().to_string() + " :: " + &message + "\n";
        let log_line = log_line.as_bytes();

        let _ = std::io::stdout().write(log_line);
    }};
}
