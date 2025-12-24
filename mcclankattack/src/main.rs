use anyhow::{Context, anyhow};

pub mod clanker;
pub mod log;
pub mod service;

enum ExitErrorCode {
    InvalidArguments = 1,
    Runtime = 2,
}

impl ExitErrorCode {
    fn exit(self) {
        std::process::exit(self as i32);
    }
}

pub struct Arguments {
    pub destination: String,
    pub number_of_threads: usize,
    pub clankers_count: usize,
    pub name_list: Option<String>,
    pub message_list: Option<String>,
    pub message_interval: u64,
}

fn hello() {
    println!(
        r#"##########################################################################
#   /$$$$$$  /$$        /$$$$$$  /$$   /$$ /$$   /$$ /$$$$$$$$ /$$$$$$$  #
#  /$$__  $$| $$       /$$__  $$| $$$ | $$| $$  /$$/| $$_____/| $$__  $$ #
# | $$  \__/| $$      | $$  \ $$| $$$$| $$| $$ /$$/ | $$      | $$  \ $$ #
# | $$      | $$      | $$$$$$$$| $$ $$ $$| $$$$$/  | $$$$$   | $$$$$$$/ #
# | $$      | $$      | $$__  $$| $$  $$$$| $$  $$  | $$__/   | $$__  $$ #
# | $$    $$| $$      | $$  | $$| $$\  $$$| $$\  $$ | $$      | $$  \ $$ #
# |  $$$$$$/| $$$$$$$$| $$  | $$| $$ \  $$| $$ \  $$| $$$$$$$$| $$  | $$ #
# \______/ |________/|__/  |__/|__/  \__/|__/  \__/|________/|__/  |__/  #
##########################################################################
"#
    )
}

fn argument_help_procedure() {
    if std::env::args().into_iter().any(|v| {
        let v = v.as_str();
        v == "-help" || v == "--help" || v == "/help" || v == "-?" || v == "--?" || v == "/?"
    }) {
        println!(
            r#"
networkings:
  --destination [HOST:PORT]   | REQUIRED | minecraft victim server

performance:
  --thread [NUMBER]           | OPTIONAL | number of threads to use, default is the current available cores

clankers:
  --clankers [NUMBER]         | REQUIRED | number of clankers to send
  --name-list [FILE PATH]     | OPTIONAL | path to file with list of names, separated by new line, default is random a-z, A-Z, 0-9
  --message-list [FILE PATH]  | REQUIRED | path to file with list of messages, separated by new line
  --message-interval [MILLIS] | REQUIRED | how many milliseconds to wait before a message is send
"#
        );

        std::process::exit(0);
    }
}

fn parse_arguments() -> anyhow::Result<Arguments> {
    macro_rules! parse_argument_para {
        ($args:expr, $arg_name:expr) => {
            if let Some(v) = $args.next() {
                v
            } else {
                return Err(anyhow!("incomplete argument '{}'", $arg_name));
            }
        };
    }

    let mut parsed_arguments = Arguments {
        destination: String::new(),
        number_of_threads: std::thread::available_parallelism()
            .expect("soooooo your OS doesn't know how much cores you have; here's an error")
            .get(),
        clankers_count: 0,
        name_list: None,
        message_list: None,
        message_interval: 0,
    };

    let mut args = std::env::args().into_iter().skip(1).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--destination" => parsed_arguments.destination = parse_argument_para!(args, arg),
            "--threads" => {
                parsed_arguments.number_of_threads = parse_argument_para!(args, arg)
                    .parse()
                    .context(anyhow!("incomplete argument '{arg}'"))?
            }
            "--clankers" => {
                parsed_arguments.clankers_count = parse_argument_para!(args, arg)
                    .parse()
                    .context(anyhow!("incomplete argument '{arg}'"))?
            }
            "--name-list" => parsed_arguments.name_list = Some(parse_argument_para!(args, arg)),
            "--message-list" => {
                parsed_arguments.message_list = Some(parse_argument_para!(args, arg))
            }
            "--message-interval" => {
                parsed_arguments.message_interval = parse_argument_para!(args, arg)
                    .parse()
                    .context(anyhow!("incomplete argument '{arg}'"))?
            }
            _ => return Err(anyhow!("unknown argument '{arg}'")),
        }
    }

    if parsed_arguments.destination.is_empty() {
        return Err(anyhow!("destination address must not be empty"));
    }

    if parsed_arguments.clankers_count == 0 {
        return Err(anyhow!("the number of clankers must not be zero"));
    }

    if parsed_arguments.message_interval == 0 {
        return Err(anyhow!("message interval must not be zero"));
    }

    if let Some(ref v) = parsed_arguments.name_list {
        if !std::fs::exists(&v)? {
            return Err(anyhow!("name list '{v}' doesn't exist on the filesystem"));
        }
    }

    if parsed_arguments.message_list.is_none() {
        return Err(anyhow!("message list is missing"));
    }

    Ok(parsed_arguments)
}

fn main() {
    hello();
    argument_help_procedure();

    let arguments = match parse_arguments() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("fatal: {}", e.to_string());
            ExitErrorCode::InvalidArguments.exit();
            return;
        }
    };

    outputln!("starting with {} threads", arguments.number_of_threads);

    match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(arguments.number_of_threads)
        .enable_all()
        .build()
    {
        Ok(rt) => {
            rt.block_on(async move {
                if let Err(e) = service::attack_loop(arguments).await {
                    errorln!("runtime error! {}", e.to_string());
                    ExitErrorCode::Runtime.exit();
                }
            });
        }
        Err(e) => {
            errorln!("failed to create MT runtime! {}", e.to_string());
            ExitErrorCode::Runtime.exit();
        }
    }
}
