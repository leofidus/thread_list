use std::time::Duration;

use memoffset::offset_of;
use windows::Win32::{
    Foundation::{CloseHandle, BOOL, FILETIME, STILL_ACTIVE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        },
        Memory::LocalFree,
        Threading::{
            GetCurrentProcessId, GetExitCodeThread, GetThreadDescription, GetThreadIOPendingFlag,
            GetThreadTimes, OpenThread, THREAD_QUERY_INFORMATION,
        },
    },
};

use crate::{Stopped, Thread, ThreadInfo, ThreadStatus};

impl TryFrom<THREADENTRY32> for Thread {
    type Error = anyhow::Error;

    fn try_from(value: THREADENTRY32) -> Result<Self, Self::Error> {
        if (value.dwSize as usize)
            < offset_of!(THREADENTRY32, th32ThreadID) + std::mem::size_of::<u32>()
        {
            return Err(anyhow::anyhow!("THREADENTRY32 struct too small"));
        }
        Ok(Thread {
            id: value.th32ThreadID,
        })
    }
}

impl Thread {
    pub fn get_info(&self) -> anyhow::Result<ThreadInfo> {
        unsafe {
            // todo: crash with Parameter is incorrect??? 0x80070057
            let handle = OpenThread(THREAD_QUERY_INFORMATION, false, self.id).unwrap();
            let mut io_pending: BOOL = Default::default();
            let io_pending = if GetThreadIOPendingFlag(handle, &mut io_pending).as_bool() {
                io_pending.as_bool()
            } else {
                return Err(std::io::Error::last_os_error().into());
            };
            let mut exit_code: u32 = 0;
            if !GetExitCodeThread(handle, &mut exit_code).as_bool() {
                return Err(std::io::Error::last_os_error().into());
            }
            let thread_name = {
                let descr = GetThreadDescription(handle)?;
                let thead_name = descr.to_string()?;
                LocalFree(descr.as_ptr() as isize);
                thead_name
            };

            let mut creation_time: FILETIME = Default::default();
            let mut exit_time: FILETIME = Default::default();
            let mut kernel_time: FILETIME = Default::default();
            let mut user_time: FILETIME = Default::default();
            if !GetThreadTimes(
                handle,
                &mut creation_time,
                &mut exit_time,
                &mut kernel_time,
                &mut user_time,
            )
            .as_bool()
            {
                return Err(std::io::Error::last_os_error().into());
            }

            Ok(ThreadInfo {
                thread: *self,
                name: thread_name,
                status: if exit_code == STILL_ACTIVE.0 as _ {
                    ThreadStatus::Running
                } else {
                    ThreadStatus::Stopped(Stopped {
                        exit_code,
                        exit_time: filetime_to_time_offsetdatetime(exit_time)?,
                    })
                },
                created: filetime_to_time_offsetdatetime(creation_time)?,
                kernel_time: filetime_to_duration(kernel_time),
                user_time: filetime_to_duration(user_time),
                io_pending,
            })
        }
    }
}

fn filetime_to_duration(time: FILETIME) -> std::time::Duration {
    // manual bitshift because alignment isn't guaranteed
    let time = ((time.dwHighDateTime as u64) << 32) + time.dwLowDateTime as u64;
    // throw away some precision to rule out overflow, micros are plenty
    Duration::from_micros(time / 10)
}

fn filetime_to_time_offsetdatetime(time: FILETIME) -> anyhow::Result<time::OffsetDateTime> {
    // manual bitshift because alignment isn't guaranteed
    let time = ((time.dwHighDateTime as i128) << 32) + time.dwLowDateTime as i128;
    // windows uses 1601-01-01 as epoch, unix 1970-01-01
    static OFFSET_TO_UNIX_TIME: i128 = 116444736000000000i128;
    let time_in_unix_epoch = time - OFFSET_TO_UNIX_TIME;
    // filetime is in 100ns intervals
    let unix_time_nanos = time_in_unix_epoch * 100;
    Ok(time::OffsetDateTime::from_unix_timestamp_nanos(
        unix_time_nanos,
    )?)
}

pub(crate) fn get_threads() -> anyhow::Result<Vec<Thread>> {
    let mut res: Vec<Thread> = Vec::new();
    unsafe {
        let process_id = GetCurrentProcessId();
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)?;

        let mut thread_entry: THREADENTRY32 = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>().try_into()?,
            ..Default::default()
        };
        if Thread32First(handle, &mut thread_entry).as_bool() {
            if get_owner_process_id(thread_entry)? == process_id {
                res.push(thread_entry.try_into()?);
            }
            while Thread32Next(handle, &mut thread_entry).as_bool() {
                if get_owner_process_id(thread_entry)? == process_id {
                    res.push(thread_entry.try_into()?);
                }
            }
        }
        CloseHandle(handle);
    }
    // log::info!("{} threads: {:#?}", res.len(), &res);
    // res.iter().for_each(|t| unsafe {
    //     let handle = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, false, t.id).unwrap();
    //     log::info!(
    //         "thread d {:?}",
    //         GetThreadDescription(handle).unwrap().to_string()
    //     );
    // });
    Ok(res)
}

fn get_owner_process_id(value: THREADENTRY32) -> anyhow::Result<u32> {
    if (value.dwSize as usize)
        < offset_of!(THREADENTRY32, th32ThreadID) + std::mem::size_of::<u32>()
    {
        return Err(anyhow::anyhow!("THREADENTRY32 struct too small"));
    }
    Ok(value.th32OwnerProcessID)
}
