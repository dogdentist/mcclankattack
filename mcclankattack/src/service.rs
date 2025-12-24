use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

use crate::{Arguments, clanker, errorln, outputln};

enum ClankerNameType {
    Generate,
    Ready(Vec<String>),
}

struct ClankerName {
    id: AtomicU64,
    names: ClankerNameType,
}

impl ClankerName {
    fn name(&mut self) -> String {
        const CHARSET: [char; 62] = [
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
            'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y',
            'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        ];

        match &self.names {
            ClankerNameType::Generate => {
                let mut name = String::with_capacity(8);

                for _ in 0..12 {
                    name.push(CHARSET[fastrand::usize(0..CHARSET.len())]);
                }

                name
            }
            ClankerNameType::Ready(list) => {
                let id: u64 = self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                list[fastrand::usize(0..list.len())].clone() + &id.to_string()
            }
        }
    }
}

pub struct ClankerMessages(Vec<String>);

impl ClankerMessages {
    pub fn message(&self) -> String {
        self.0[fastrand::usize(0..self.0.len())].clone()
    }
}

async fn clanker(
    username: &str,
    destination: &str,
    message_interval: u64,
    clanker_messages: Arc<ClankerMessages>,
) -> anyhow::Result<()> {
    let (conn, clanker) =
        clanker::Clanker::new(username.to_owned(), destination.to_owned()).await?;
    let (conn_rx, conn_tx) = conn.into_split();
    let mut conn_rx: clanker::ClankerIo<OwnedReadHalf> =
        clanker::ClankerIo::new(conn_rx, clanker.clone());
    let mut conn_tx: Arc<Mutex<clanker::ClankerIo<OwnedWriteHalf>>> = Arc::new(Mutex::new(
        clanker::ClankerIo::new(conn_tx, clanker.clone()),
    ));

    clanker::join_game(clanker.clone(), &mut conn_rx, &mut conn_tx).await?;

    outputln!("clanker '{username}' joined the game");

    let spammer_thread = {
        let username = username.to_owned();
        let mut conn_tx = conn_tx.clone();

        tokio::spawn(async move {
            if let Err(e) =
                clanker::spam_messages(&mut conn_tx, message_interval, clanker_messages).await
            {
                errorln!(
                    "username '{username}' crashed on sending a message, error: {}",
                    e.to_string()
                );
            }
        })
    };

    let game_result = clanker::game_handler(&mut conn_rx, &mut conn_tx).await;
    spammer_thread.abort();

    return game_result;
}

pub async fn attack_loop(args: Arguments) -> anyhow::Result<()> {
    outputln!("attacking '{}'", args.destination);

    let clanker_name = Arc::new(Mutex::new(if let Some(name_list_path) = args.name_list {
        let bot_names: Vec<String> = std::fs::read_to_string(name_list_path)?
            .lines()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .collect();

        ClankerName {
            id: AtomicU64::new(0),
            names: ClankerNameType::Ready(bot_names),
        }
    } else {
        ClankerName {
            id: AtomicU64::new(0),
            names: ClankerNameType::Generate,
        }
    }));

    let clanker_messages = Arc::new({
        let bot_messages: Vec<String> =
            std::fs::read_to_string(args.message_list.expect("message_list is None"))?
                .lines()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string())
                .collect();

        ClankerMessages(bot_messages)
    });

    let mut clankers_threads = Vec::new();

    for _ in 0..args.clankers_count {
        let clanker_name = clanker_name.clone();
        let clanker_messages = clanker_messages.clone();
        let destination = args.destination.clone();

        clankers_threads.push(tokio::spawn(async move {
            loop {
                let clanker_messages = clanker_messages.clone();
                let username = clanker_name.lock().await.name();

                if let Err(e) = clanker(
                    &username,
                    &destination,
                    args.message_interval,
                    clanker_messages,
                )
                .await
                {
                    errorln!("clanker '{username}' died :pray:, error: {}", e.to_string());
                }
            }
        }));
    }

    for clanker in clankers_threads {
        let _ = clanker.await;
    }

    Ok(())
}
