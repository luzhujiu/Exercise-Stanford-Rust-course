use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::process::{Child, Command};
use std::os::unix::process::CommandExt;
use std::fmt;
use crate::dwarf_data::{DwarfData, Line};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::fs;

#[derive(Clone, Debug)]
pub struct Breakpoint {
    addr: usize,
    orig_byte: u8,
}

#[derive(Debug)]
pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Status::Stopped(sig1,_), Status::Stopped(sig2,_)) => sig1 == sig2,
            (Status::Exited(sig1), Status::Exited(sig2)) => sig1 == sig2,
            (Status::Signaled(sig1), Status::Signaled(sig2)) => sig1 == sig2,
            _ => false,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Stopped(signal, _) => write!(f, "Child stopped (signal {})", signal.to_string()),
            Status::Exited(code) => write!(f, "Child exited (status = {})", code),
            Status::Signaled(signal) => write!(f, "Child signaled (signal {})", signal.to_string()),
        }
    }
}

impl Status {
    pub fn print(&self, debug_data: &DwarfData, lineinfo: &mut HashMap<String, Vec<String>>) {
        println!("{}", self);
        match self {
            Status::Stopped(_, rip) => {
                let line = debug_data.get_line_from_addr(*rip);
                let name = debug_data.get_function_from_addr(*rip);
                if line.is_some() && name.is_some() {
                    let line = line.unwrap();
                    let name = name.unwrap();
                    println!("Stopped at {} ({}:{})", name, line.file, line.number);
                    if let Some(lines) = lineinfo.get(&line.file) {
                        println!("{}", lines[line.number-1]);
                    } else {
                        let reader = BufReader::new(fs::File::open(&line.file).unwrap());
                        let lines = reader.lines().map(|l| l.unwrap()).collect::<Vec<String>>();
                        println!("{}", lines[line.number-1]);
                        lineinfo.insert(line.file, lines);
                    }

                } else {
                    println!("can not print");
                }
            },
            _ => {
                println!("can not print");
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    NixError(nix::Error),
    IOError(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Error {
        Error::IOError(error)
    }
}

impl From<nix::Error> for Error {
    fn from(error: nix::Error) -> Error {
        Error::NixError(error)
    }
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

use std::mem::size_of;
fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

pub struct Inferior {
    child: Child,
    breakpoints: HashMap<usize, Breakpoint>,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>, breakpoints: &Vec<usize>) -> Option<Inferior> {
        let child = unsafe { Command::new(target)
            .args(args)
            .pre_exec(child_traceme)
            .spawn()
            .ok()? };

        let mut me = Inferior { child, breakpoints: HashMap::new() };
        
        let status = me.wait(None).ok()?;

        for addr in breakpoints {
            me.set_breakpoint(*addr);
        }
        
        if status == Status::Stopped(signal::Signal::SIGTRAP, 0) {
            return Some(me);
        } else {
            return None;
        }
    }
    
    /// set break point
    pub fn set_breakpoint(&mut self, addr: usize) {
        if let Ok(orig_byte) = self.write_byte(addr, 0xcc) { 
            self.breakpoints.insert(addr, Breakpoint{ addr, orig_byte });
        }
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => {
                Status::Exited(exit_code)
            },
            WaitStatus::Signaled(_pid, signal, _core_dumped) => {
                Status::Signaled(signal)
            },
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            },
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn step(&mut self, dwarf: &DwarfData) -> Result<Status, nix::Error> {
        let mut regs = ptrace::getregs(self.pid())?;
        let rip = regs.rip as usize;

        if self.breakpoints.contains_key(&rip) {
            ptrace::step(self.pid(), None)?;
            let status = self.wait(None)?;
            if status == Status::Stopped(signal::Signal::SIGTRAP,0) {
                let breakpoint = self.breakpoints.get(&rip).unwrap().to_owned();    
                self.write_byte(breakpoint.addr, 0xcc)?;
            } 
            return Ok(status);
        }

        ptrace::step(self.pid(), None)?;
        let status = self.wait(None)?;

        if let Status::Stopped(signal::Signal::SIGTRAP, rip) = status {
            if self.breakpoints.contains_key(&(rip-1)) {
                let breakpoint = self.breakpoints.get(&(rip-1)).unwrap().to_owned();
                self.write_byte(breakpoint.addr, breakpoint.orig_byte)?;
                let mut regs = ptrace::getregs(self.pid())?;
                regs.rip -= 1;
                ptrace::setregs(self.pid(), regs)?;
            }
        } 
        
        return Ok(status);
    }

    pub fn next(&mut self, dwarf: &DwarfData) -> Result<Status, nix::Error> {
        let mut regs = ptrace::getregs(self.pid())?;
        let rip = regs.rip as usize;
        let current_line = dwarf.get_line_from_addr(rip).expect("should have line number");
        
        loop {
            let status = self.step(dwarf)?;
            if let Status::Stopped(signal::Signal::SIGTRAP, rip) = status {
                if let Some(line) = dwarf.get_line_from_addr(rip) {
                    if current_line.to_string() != line.to_string() {
                        return Ok(status);
                    }
                }
            } else {
                return Ok(status);
            }
        }
    }

    pub fn continuee(&mut self) -> Result<Status, nix::Error> {
        let mut regs = ptrace::getregs(self.pid())?;
        let rip = regs.rip as usize;

        if self.breakpoints.contains_key(&rip) {
            ptrace::step(self.pid(), None)?;
            let status = self.wait(None)?;
            if status == Status::Stopped(signal::Signal::SIGTRAP,0) {  
                let breakpoint = self.breakpoints.get(&rip).unwrap().to_owned();  
                self.write_byte(breakpoint.addr, 0xcc)?;
            } else {
                return Ok(status);
            }
        }

        ptrace::cont(self.pid(), None)?;
        let status = self.wait(None)?;

        if let Status::Stopped(signal::Signal::SIGTRAP, rip) = status {
            if self.breakpoints.contains_key(&(rip-1)) {
                let breakpoint = self.breakpoints.get(&(rip-1)).unwrap().to_owned();
                self.write_byte(breakpoint.addr, breakpoint.orig_byte)?;
                let mut regs = ptrace::getregs(self.pid())?;
                regs.rip -= 1;
                ptrace::setregs(self.pid(), regs)?;
            }
        } 

        return Ok(status);
    }
    

    pub fn kill(&mut self) -> Result<Status, Error> {
        self.child.kill()?;
        let status = self.wait(None)?;
        Ok(status)
    }

    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let mut rip: usize = regs.rip as usize;
        let mut rbp: usize = regs.rbp as usize;

        loop {
            let line: Line = debug_data.get_line_from_addr(rip).expect("get_line_from_addr fail.");
            let name = debug_data.get_function_from_addr(rip).expect("get_func_from_addr fail.");
            println!("{} ({}:{})", name, line.file, line.number);
            if name == "main" {
                break;
            }
            rip = ptrace::read(self.pid(), (rbp + 8) as ptrace::AddressType)? as usize;
            rbp = ptrace::read(self.pid(), rbp as ptrace::AddressType)? as usize;
        }

        Ok(())
    }

    fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }

    fn print_word(&self, addr: usize) -> Result<(), nix::Error>{
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        println!("{:#x}", word);
        Ok(())
    }
}
