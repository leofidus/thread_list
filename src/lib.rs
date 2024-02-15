use std::time::Duration;

use derivative::Derivative;

mod platform;

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Thread {
    id: u32,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ThreadInfo {
    #[derivative(Debug(format_with = "fmt_thread"))]
    thread: Thread,
    name: String,
    status: ThreadStatus,
    #[derivative(Debug(format_with = "fmt_offsetdatetime"))]
    created: time::OffsetDateTime,
    kernel_time: Duration,
    user_time: Duration,
    io_pending: bool,
}

#[derive(Debug)]
pub enum ThreadStatus {
    Running,
    Stopped(Stopped),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Stopped {
    exit_code: u32,
    #[derivative(Debug(format_with = "fmt_offsetdatetime"))]
    exit_time: time::OffsetDateTime,
}

/// Get all threads of the current process
pub fn get_threads() -> anyhow::Result<Vec<Thread>> {
    platform::get_threads()
}

fn fmt_offsetdatetime(
    val: &time::OffsetDateTime,
    fmt: &mut std::fmt::Formatter,
) -> Result<(), std::fmt::Error> {
    // val.format_into(fmt, &time::format_description::well_known::Rfc3339)
    fmt.write_fmt(format_args!(
        "{}",
        val.format(&time::format_description::well_known::Rfc3339)
            .unwrap()
    ))
}

fn fmt_thread(val: &Thread, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    fmt.write_fmt(format_args!("{}", val.id))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
