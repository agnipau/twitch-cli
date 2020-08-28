mod api;

use api::{Chatters, Clips, Comments, UserData, Vod, Vods};
use clap::{App, AppSettings, Arg, SubCommand};

#[tokio::main]
async fn main() {
    let matches = App::new("Twitch-CLI")
        .version("0.1")
        .author("Matteo Guarda <matteoguarda@tutanota.com>")
        .about("A CLI for twitch that does things that can't be done with the web interface")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("infos")
                .about("Shows infos about an user")
                .arg(
                    Arg::with_name("USERNAME")
                        .required(true)
                        .help("Username of the user to show infos about"),
                ),
        )
        .subcommand(
            SubCommand::with_name("dl")
                .about("Logs the direct link to a VOD")
                .arg(
                    Arg::with_name("VOD_ID")
                        .required(true)
                        .help("ID of the VOD"),
                ),
        )
        .subcommand(
            SubCommand::with_name("m3u8-gen")
                .about("Generate the M3U8 of an entire Twitch VOD or a part of it")
                .arg(
                    Arg::with_name("VOD_ID")
                        .required(true)
                        .help("ID of the VOD"),
                )
                .arg(
                    Arg::with_name("start")
                        .short("s")
                        .value_name("START")
                        .help("Start duration in seconds")
                        .default_value("0.0"),
                )
                .arg(
                    Arg::with_name("end")
                        .short("e")
                        .value_name("END")
                        .help("End duration in seconds"),
                ),
        )
        .subcommand(
            SubCommand::with_name("vods")
                .about("Shows all the vods of an user")
                .arg(
                    Arg::with_name("USERNAME")
                        .required(true)
                        .help("Username of the user who posted the vods"),
                )
                .arg(
                    Arg::with_name("iterations")
                        .short("i")
                        .value_name("ITERATIONS")
                        .help("Number of iterations to do"),
                )
                .arg(
                    Arg::with_name("cursor")
                        .short("c")
                        .value_name("CURSOR")
                        .help("Cursor, used to start fetching from a certain point on"),
                ),
        )
        .subcommand(
            SubCommand::with_name("clips")
                .about("Shows all the clips of an user between a range of time")
                .arg(
                    Arg::with_name("USERNAME")
                        .required(true)
                        .help("Username of the user who posted the clips"),
                )
                .arg(
                    Arg::with_name("STARTED_AT")
                        .required(true)
                        .help("Start of the range (RFC3339 format) (e.g. 2020-03-26T00:00:00Z)"),
                )
                .arg(
                    Arg::with_name("ENDED_AT")
                        .required(true)
                        .help("End of the range (RFC3339 format) (e.g. 2020-04-26T00:00:00Z)"),
                )
                .arg(
                    Arg::with_name("iterations")
                        .short("i")
                        .value_name("ITERATIONS")
                        .help("Number of iterations to do"),
                )
                .arg(
                    Arg::with_name("cursor")
                        .short("c")
                        .value_name("CURSOR")
                        .help("Cursor, used to start fetching from a certain point on"),
                ),
        )
        .subcommand(
            SubCommand::with_name("comments")
                .about("Shows all the comments of a VOD")
                .arg(
                    Arg::with_name("VOD_ID")
                        .required(true)
                        .help("ID of the VOD"),
                )
                .arg(
                    Arg::with_name("iterations")
                        .short("i")
                        .value_name("ITERATIONS")
                        .help("Number of iterations to do"),
                )
                .arg(
                    Arg::with_name("cursor")
                        .short("c")
                        .value_name("CURSOR")
                        .help("Cursor, used to start fetching from a certain point on"),
                ),
        )
        .subcommand(
            SubCommand::with_name("chatters")
                .about("List all the online catters given a streamer's username")
                .arg(
                    Arg::with_name("STREAMER_USERNAME")
                        .required(true)
                        .help("Username of the streamer"),
                )
                .arg(
                    Arg::with_name("USERNAME")
                        .multiple(true)
                        .help("Username of the chatter. Accepts multiple values. If given the program outputs if USERNAME is watching STREAMER_USERNAME"),
                ),
        )
        .subcommand(
            SubCommand::with_name("are-live")
                .about("Given a list of streamers, check if they are live")
                .arg(
                    Arg::with_name("USERNAME")
                        .required(true)
                        .multiple(true)
                        .help("Username of the streamer"),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("infos") {
        let username = matches.value_of("USERNAME").unwrap();
        let user_data = UserData::new(username).await.ok();
        println!("{}", serde_json::to_string(&user_data).unwrap());
        return;
    }

    if let Some(matches) = matches.subcommand_matches("dl") {
        let vod_id = matches.value_of("VOD_ID").unwrap();
        if let Ok(direct_link) = Vod::fetch_direct_link(&vod_id).await {
            println!("{}", direct_link);
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("are-live") {
        let usernames = matches
            .values_of("USERNAME")
            .unwrap()
            .collect::<Vec<&str>>();
        let are_live = UserData::are_live(&usernames).await;
        for (idx, is_live) in are_live.into_iter().enumerate() {
            println!("{}: {}", usernames[idx], is_live);
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("chatters") {
        let streamer = matches.value_of("STREAMER_USERNAME").unwrap();
        let usernames = matches
            .values_of("USERNAME")
            .map(|values| values.collect::<Vec<_>>())
            .unwrap_or(Vec::new());

        if let Ok(chatters) = Chatters::new(streamer).await {
            if usernames.len() == 0 {
                println!("{}", serde_json::to_string(&chatters).unwrap());
                return;
            }
            for (idx, is_online) in chatters.are_online(&usernames).into_iter().enumerate() {
                println!("{}: {}", usernames[idx], is_online);
            }
        } else {
            eprintln!("There is not a Twitch user named `{}`", streamer);
            std::process::exit(1);
        }
    }

    if let Some(matches) = matches.subcommand_matches("m3u8-gen") {
        let vod_id = matches.value_of("VOD_ID").unwrap();
        let start = matches
            .value_of("start")
            .map(|x| x.parse().expect("start parameter is not a valid float"));
        let end = matches
            .value_of("end")
            .map(|x| x.parse().expect("end parameter is not a valid float"));

        if let Ok(m3u8) = Vod::m3u8_gen(&vod_id, start, end).await {
            println!("{}", m3u8);
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("vods") {
        let username = matches.value_of("USERNAME").unwrap();
        let iterations = matches
            .value_of("iterations")
            .and_then(|x| x.parse().ok())
            .unwrap_or(std::u64::MAX);
        let mut cursor = matches.value_of("cursor").map(|x| x.to_string());

        let mut i = 0;
        loop {
            if let Ok(vods) = Vods::new(&username, cursor.as_deref()).await {
                println!("{}", serde_json::to_string(&vods.vods).unwrap());
                cursor = vods.cursor.clone();
                i += 1;
            } else {
                break;
            }

            if cursor.is_none() || i == iterations {
                break;
            }
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("clips") {
        let username = matches.value_of("USERNAME").unwrap();
        let start = matches.value_of("STARTED_AT").unwrap();
        let end = matches.value_of("ENDED_AT").unwrap();
        let iterations = matches
            .value_of("iterations")
            .and_then(|x| x.parse().ok())
            .unwrap_or(std::u64::MAX);
        let mut cursor = matches.value_of("cursor").map(|x| x.to_string());

        let mut i = 0;
        loop {
            if let Ok(clips) = Clips::new(&username, &start, &end, cursor.as_deref()).await {
                println!("{}", serde_json::to_string(&clips.clips).unwrap());
                cursor = clips.cursor.clone();
                i += 1;
            } else {
                break;
            }

            if cursor.is_none() || i == iterations {
                break;
            }
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("comments") {
        let vod_id = matches.value_of("VOD_ID").unwrap();
        let iterations = matches
            .value_of("iterations")
            .and_then(|x| x.parse().ok())
            .unwrap_or(std::u64::MAX);
        let mut cursor = matches.value_of("cursor").map(|x| x.to_string());

        let mut i = 0;
        loop {
            if let Ok(comments) = Comments::new(&vod_id, cursor.as_deref()).await {
                println!("{}", serde_json::to_string(&comments.comments).unwrap());
                cursor = comments.cursor.clone();
                i += 1;
            } else {
                break;
            }

            if cursor.is_none() || i == iterations {
                break;
            }
        }
    }
}
