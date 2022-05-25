# Introduction

This is a software I made back in 2020 to remotely control a minecraft server I had via Telegram. **The purpose was to learn rust in the process**, which led me to reinvent the wheel and do some strange things. **It is neither optimized, secure nor prepared to be run in production** (after a while, I switched to K8s using rcon for remote administration).

Features:

- Sends the minecraft server output to you as Telegram messages (redirects stdout to Telegram)
- Allows you to send commands to the minecraft server (redirects Telegram to stdin)
- Allows you to remotely send a special backup command that stops the minecraft server, makes a backup and starts it again.

WARNING: only available for Linux servers

# Installation steps

1. Create a Telegram Bot using the BotFather to get an API key
2. Using your Telegram account, send a message to your bot so that Telegram allows it to contact you in the future
3. Get your Telegram user id 
4. Compile the program running the following command

```sh
cargo build --release
```
The output will be a single executable file which we will call r_server_manager.

5. Prepare a config.ini file. There is an example of configuration file in this repository with comments.
6. Prepare a directory structure similar to this one
    
        ├── r_server_manager            # Remote server manager executable file
        ├── config.ini               	# Configuration file
        ├── backups                     # Directory for storing backups
        ├── server                      # Directory with the minecraft server files
        │    ├── ...  
    
7. Move to the main directory and execute the manager with the command
```sh
./r_server_manager
```

# Usage

Once the manager is running, the stdout of the manager is sent to you as Telegram messages via the Telegram Bot you configured and anything you send to the Telegram Bot will be redirected to the manager's stdin. 

Output lines that start with [ERROR], [WARN], [INFO] or [DEBUG] come from the the manager, while messages that start with [MINECRAFT] are the output of the minecraft server (which is spawned as a child process of the manager).

Everything you write as input to the manager will be sent as input to the minecraft server excep the two special commands that are interpreted by the manager itself: backup and stop. The "backup" command stops the server, saves a backup of to the directory that was specified in the config.ini file and and starts the server again. The "stop" command overwrites the minecraft "stop" command and stops both the server and the manager in a structured way.
