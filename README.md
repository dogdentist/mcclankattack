# mcclankattack

'mcclankattack' is a Minecraft bot spammer. It connects bots that spam messages—nothing special. So, randomly, I got bored near Christmas and wanted to make one—like the old days.

### how to run?

You need a file of messages that will be spammed; each message must be on a new line—this is required, not optional. The argument for this is '--message-list', and you can specify the interval of messages using '--message-interval {milliseconds}'.

Optionally, you can specify a list of usernames to use; by default, it will append an index as a suffix to the name. The argument for this is '--name-list [file path]'.

The other arguments are '--clankers {number}'—the number of bots to use; '--threads'—the number of threads to use, by default is the number of available cores; and '--destination {ip/fqdn:port}'—the remote address of the Minecraft server.

Overall, you can check the list of commands by running with an argument '--help'.

![mc bot cli example](/mcclankattack-cli.png)
![mc bot spam example](/mcclankattack-spam.gif)

```sh
cargo run -- --destination localhost:25565 --clankers 19  --message-list messages.txt --message-interval 200 
```

### note

This project is for educational purposes. It's not meant to be an attack project—you will notice this by the lack of features it needs to bypass the lack of protection.
