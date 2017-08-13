extern crate clap;

use std::fs::File;
use std::io::{stdin, stdout};
use std::path::Path;

use clap::{App, Arg, ArgMatches, SubCommand};

#[allow(dead_code)]
struct ProcStat {
    comm: String,
    pid: u32,
    ppid: u32,
    state: char,
}

impl ProcStat {
    pub fn read_pid(pid: u32) -> Option<ProcStat> {
        let stat = {
            let path = format!("/proc/{}/stat", pid);
            match string_from_path(&path) {
                Some(s) => s,
                None => {
                    return None;
                }
            }
        };

        let (comm, stat_end) = {
            let lparen = stat.find('(').unwrap();
            let rparen = stat.rfind(')').unwrap();

            (&stat[(lparen + 1)..rparen], &stat[(rparen + 2)..])
        };

        let mut pieces = stat_end.split(' ');

        let state = match pieces.next() {
            Some(s) => s.chars().next().unwrap(),
            None => {
                return None;
            }
        };

        let ppid = pieces.next().unwrap().parse::<u32>().unwrap();

        Some(ProcStat {
            comm: String::from(comm),
            pid: pid,
            ppid: ppid,
            state: state,
        })
    }
}

fn bytes_from_path<P: AsRef<Path>>(path: P) -> Option<Vec<u8>> {
    use std::io::Read;

    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            return None;
        }
    };

    let mut buf = Vec::new();

    if file.read_to_end(&mut buf).is_err() {
        return None;
    }

    Some(buf)
}

fn cmdline_to_stdout(pid: u32) {
    /* It is a bit ugly to couple two distinct I/O actions like this
     * but this lets us neatly bypass decoding and encoding stuff. */
    use std::io::Write;

    let cmdline = {
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        bytes_from_path(&cmdline_path).expect("requested process has to exist")
    };

    let mut pieces = cmdline[..cmdline.len() - 1].split(|b| *b == 0);

    let first = pieces.next().expect("need to have some argument");
    let separator: &[u8] = " \\\n    ".as_bytes();

    let stdout = stdout();
    let mut out_lock = stdout.lock();
    out_lock.write(first).unwrap();

    for piece in pieces {
        out_lock.write(separator).unwrap();
        out_lock.write(piece).unwrap();
    }
    out_lock.write(b"\n").unwrap();
}

#[allow(dead_code)]
fn format_arglist(args: &[&str]) -> String {
    args.join(" \\\n    ")
}

fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Joonas Sarajärvi <muep@iki.fi>")
        .about("A yet another process info tool")
        .subcommand(
            SubCommand::with_name("args")
                .about("Print out args of a running process")
                .arg(
                    Arg::with_name("pid")
                        .short("p")
                        .help("select process by id")
                        .value_name("PID")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(SubCommand::with_name("prettify").about(
            "Reprint an argument list for easier viewing",
        ))
        .subcommand(
            SubCommand::with_name("whatps")
                .about("Print out args and parents of a running process")
                .arg(
                    Arg::with_name("pid")
                        .short("p")
                        .help("select process by id")
                        .value_name("PID")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .get_matches();

    if let Some(ref m) = matches.subcommand_matches("args") {
        run_args(m);
    } else if matches.subcommand_matches("prettify").is_some() {
        run_prettify();
    } else if let Some(ref m) = matches.subcommand_matches("whatps") {
        run_whatps(m);
    }

}

fn prettify(stuff_in: &str) -> String {
    let pieces: Vec<&str> = stuff_in.split(' ').collect();

    pieces.join(" \\\n    ")
}

fn run_args(matches: &ArgMatches) {
    let pid = matches.value_of("pid").unwrap().parse::<u32>().expect(
        "PID has to be an integer",
    );

    cmdline_to_stdout(pid);
}

fn run_prettify() {
    use std::io::Read;

    let stdin_all = {
        let mut buf = String::new();
        stdin().read_to_string(&mut buf).unwrap();
        buf
    };

    println!("{}", prettify(&stdin_all))
}

fn run_whatps(matches: &ArgMatches) {
    let mut pid = matches.value_of("pid").unwrap().parse::<u32>().expect(
        "PID has to be an integer",
    );

    let mut pids = vec![pid];

    while pid != 1 {
        let stat = match ProcStat::read_pid(pid) {
            Some(s) => s,
            None => {
                break;
            }
        };

        pid = stat.ppid;
        pids.push(pid);
    }

    for pid in pids.iter().rev() {
        let state = match ProcStat::read_pid(*pid) {
            Some(stat) => stat.state,
            None => '?',
        };

        println!("\npid {} [{}]:", pid, state);

        cmdline_to_stdout(*pid);
    }
}

fn string_from_path<P: AsRef<Path>>(path: P) -> Option<String> {
    use std::io::Read;

    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            return None;
        }
    };

    let mut buf = String::new();

    if file.read_to_string(&mut buf).is_err() {
        return None;
    }

    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_arglist_1() {
        let args = ["gcc", "-c", "hello.c"];

        let expected_pretty = include_str!("td/format_arglist_1.txt");
        let prettified = format_arglist(&args);

        assert_eq!(expected_pretty, &prettified);
    }

    #[test]
    fn test_prettify_1() {
        let text = include_str!("td/prettify_1_orig.txt");
        let expected_pretty = include_str!("td/prettify_1_pretty.txt");

        let prettified = prettify(text);

        assert_eq!(expected_pretty, &prettified);
    }
}
