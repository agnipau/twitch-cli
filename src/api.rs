use futures::future;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

const OAUTH_TOKEN: &str = "jbun01lt3ul2yufhudh2m4m6ncokg3";
const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const VODS_HASH: &str = "c3306aa37d92b24bc81a9b28dc64fca8232d53bc3072cd7038c71c0e704c0f58";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/81.0.4044.113 Safari/537.36";

lazy_static! {
    static ref SEGMENT_RE: Regex = Regex::new(
        r#"#EXTINF:(?P<segmentDuration>[0-9]+\.[0-9]{3}),\n(?P<segmentNum>[0-9]+)\.ts"#,
    )
    .unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserData {
    id: String,
    login: String,
    display_name: String,
    #[serde(rename(deserialize = "type"))]
    user_type: String,
    broadcaster_type: String,
    description: String,
    profile_image_url: String,
    offline_image_url: String,
    view_count: u64,
}

impl UserData {
    pub async fn new(username: &str) -> Result<UserData, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        Ok(serde_json::from_value(
            client
                .get(&format!(
                    "https://api.twitch.tv/helix/users?login={}",
                    username
                ))
                .header("Client-Id", CLIENT_ID)
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .header(
                    reqwest::header::AUTHORIZATION,
                    &format!("Bearer {}", OAUTH_TOKEN),
                )
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?["data"]
                .as_array()
                .ok_or(Box::<dyn std::error::Error>::from(
                    "Twitch API was expected to return an array and didn't return one",
                ))
                .map(|x| x[0].clone())?,
        )?)
    }

    pub async fn is_live(channel_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!(
                "https://api.twitch.tv/helix/streams?user_login={}",
                channel_name
            ))
            .header(reqwest::header::ACCEPT, "application/vnd.twitchtv.v5+json")
            .header(
                reqwest::header::AUTHORIZATION,
                &format!("Bearer {}", OAUTH_TOKEN),
            )
            .header("Client-Id", CLIENT_ID)
            .send()
            .await?
            .text()
            .await?;
        let ch = resp.chars().collect::<Vec<_>>()[9];
        return Ok(ch != ']');
    }

    pub async fn are_live(channel_names: &[&str]) -> Vec<bool> {
        future::join_all(
            channel_names
                .into_iter()
                .map(|user| Self::is_live(user))
                .collect::<Vec<_>>(),
        )
        .await
        .into_iter()
        .filter_map(|x| x.ok())
        .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Chatters {
    chatter_count: u64,
    chatters: ChattersChatters,
}

impl Chatters {
    pub async fn new(username: &str) -> reqwest::Result<Chatters> {
        let client = reqwest::Client::new();
        client
            .get(&format!(
                "https://tmi.twitch.tv/group/user/{}/chatters",
                username
            ))
            .send()
            .await?
            .json()
            .await
    }

    pub fn is_online(&self, username: &str) -> bool {
        let u = &username.to_string();
        self.chatters.broadcaster.contains(u)
            || self.chatters.vips.contains(u)
            || self.chatters.moderators.contains(u)
            || self.chatters.staff.contains(u)
            || self.chatters.admins.contains(u)
            || self.chatters.global_mods.contains(u)
            || self.chatters.viewers.contains(u)
    }

    pub fn are_online<'a>(&self, usernames: &'a [&str]) -> Vec<bool> {
        usernames
            .into_iter()
            .map(|username| self.is_online(username))
            .collect::<Vec<_>>()
    }
}

#[derive(Serialize, Deserialize)]
struct ChattersChatters {
    broadcaster: Vec<String>,
    vips: Vec<String>,
    moderators: Vec<String>,
    staff: Vec<String>,
    admins: Vec<String>,
    global_mods: Vec<String>,
    viewers: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Clip {
    id: String,
    url: String,
    embed_url: String,
    broadcaster_id: String,
    broadcaster_name: String,
    creator_id: String,
    creator_name: String,
    video_id: String,
    game_id: String,
    language: String,
    title: String,
    view_count: u64,
    created_at: String,
    thumbnail_url: String,
}

#[derive(Debug)]
pub struct Clips {
    pub clips: Vec<Clip>,
    pub cursor: Option<String>,
}

impl Clips {
    pub async fn new(
        username: &str,
        started_at: &str,
        ended_at: &str,
        cursor: Option<&str>,
    ) -> Result<Clips, Box<dyn std::error::Error>> {
        let broadcaster_id = {
            let user_data = UserData::new(username).await?;
            user_data.id
        };

        let client = reqwest::Client::new();
        let resp = client
            .get(&format!(
                "https://api.twitch.tv/helix/clips?broadcaster_id={}&started_at={}&ended_at={}&after={}",
                broadcaster_id,
                started_at,
                ended_at,
                cursor.unwrap_or("").to_string()
            ))
            .header("Client-Id", CLIENT_ID)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header(
                reqwest::header::AUTHORIZATION,
                &format!("Bearer {}", OAUTH_TOKEN),
            )
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(Clips {
            cursor: resp["pagination"]["cursor"].as_str().map(|x| x.to_string()),
            clips: resp["data"]
                .as_array()
                .ok_or(Box::<dyn std::error::Error>::from(
                    "Error decoding 'data', expecting Array, got something else",
                ))?
                .into_iter()
                .filter_map(|item| serde_json::from_value(item.clone()).ok())
                .collect(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Vod {
    id: String,
    #[serde(rename(deserialize = "lengthSeconds"))]
    length_seconds: u64,
    #[serde(rename(deserialize = "publishedAt"))]
    published_at: String,
    #[serde(rename(deserialize = "viewCount"))]
    view_count: u64,
    title: String,
    #[serde(skip_deserializing)]
    url: String,
}

#[derive(Debug)]
struct Segment {
    uri: String,
    duration: f64,
}

impl Vod {
    pub async fn fetch_direct_link(vodid: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!("https://api.twitch.tv/api/vods/{}/access_token?need_https=true&oauth_token=&platform=_&player_backend=mediaplayer&player_type=site", vodid))
            .header("Client-Id", CLIENT_ID)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let sig = resp["sig"].as_str().map(|x| x.to_string()).ok_or(
            Box::<dyn std::error::Error>::from(
                "Error decoding 'sig', expecting String, got something else",
            ),
        )?;
        let token = resp["token"].as_str().map(|x| x.to_string()).ok_or(Box::<
            dyn std::error::Error,
        >::from(
            "Error decoding 'token', expecting String, got something else",
        ))?;
        Ok(format!("https://usher.ttvnw.net/vod/{}.m3u8?allow_source=true&player_backend=mediaplayer&playlist_include_framerate=true&reassignments_supported=true&sig={}&supported_codecs=avc1&token={}&cdm=wv&player_version=0.9.8", vodid, sig, token))
    }

    pub async fn m3u8_gen(
        vodid: &str,
        start: Option<f64>,
        end: Option<f64>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let start = start.unwrap_or(0.0);
        let end = end.unwrap_or(std::f64::INFINITY);

        let m3u8_link = Self::fetch_direct_link(vodid).await?;
        let m3u8_content = reqwest::get(&m3u8_link).await?.text().await?;
        let direct_link = {
            let parts = m3u8_content
                .split("https://")
                .nth(1)
                .unwrap()
                .split(".m3u8")
                .nth(0)
                .unwrap();
            format!("https://{}.m3u8", parts)
        };
        let direct_link_parent = Path::new(&direct_link).parent().unwrap().to_str().unwrap();
        let direct_link_m3u8_content = reqwest::get(&direct_link).await?.text().await?;

        let segments = {
            let segments: Vec<_> = SEGMENT_RE
                .captures_iter(&direct_link_m3u8_content)
                .into_iter()
                .map(|caps| Segment {
                    duration: caps["segmentDuration"].parse().unwrap(),
                    uri: format!("{}/{}.ts", direct_link_parent, &caps["segmentNum"]),
                })
                .collect();

            let mut duration_counter = 0.0;
            let mut final_segments = vec![];
            for segment in segments {
                duration_counter += segment.duration;
                if duration_counter >= start && duration_counter <= end {
                    final_segments.push(segment);
                } else if duration_counter > end {
                    break;
                }
            }
            final_segments
        };

        let mut final_m3u8_content = "
#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:12
#EXT-X-PLAYLIST-TYPE:EVENT
#EXT-X-MEDIA-SEQUENCE:0"
            .trim_start()
            .to_string();
        for segment in segments {
            final_m3u8_content
                .push_str(&format!("#EXTINF:{},\n{}\n", segment.duration, segment.uri));
        }
        final_m3u8_content.push_str("#EXT-X-ENDLIST");
        Ok(final_m3u8_content)
    }
}

#[derive(Debug)]
pub struct Vods {
    pub vods: Vec<Vod>,
    pub cursor: Option<String>,
}

impl Vods {
    pub async fn new(
        username: &str,
        cursor: Option<&str>,
    ) -> Result<Vods, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        let resp = client
            .post("https://gql.twitch.tv/gql")
            .body(format!(
                "[{{\"operationName\":\"FilterableVideoTower_Videos\",\"variables\":{{\"limit\":30,\"channelOwnerLogin\":\"{}\",\"broadcastType\":\"ARCHIVE\",\"videoSort\":\"TIME\",\"cursor\":\"{}\"}},\"extensions\":{{\"persistedQuery\":{{\"version\":1,\"sha256Hash\":\"{}\"}}}}}}]",
                username,
                cursor.unwrap_or(""),
                VODS_HASH
            ))
            .header(reqwest::header::ACCEPT, "*/*")
            .header(reqwest::header::ACCEPT_LANGUAGE, "it-IT")
            .header("Client-Id", CLIENT_ID)
            .header(reqwest::header::CONNECTION, "keep-alive")
            .header(reqwest::header::CONTENT_TYPE, "text/plain;charset=UTF-8")
            .header(reqwest::header::HOST, "gql.twitch.tv")
            .header(reqwest::header::ORIGIN, "https://www.twitch.tv")
            .header(reqwest::header::REFERER, &format!("https://www.twitch.tv/{}/videos?filter=archives&sort=time", username))
            .header("Sec-Fetch-Dest", "empty")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Site", "same-site")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let user = resp.as_array().ok_or(Box::<dyn std::error::Error>::from(
            "Twitch API was expected to return an array and didn't return one",
        ))?[0]["data"]["user"]
            .clone();
        if user.is_null() {
            return Err(Box::<dyn std::error::Error>::from("User doesn't exist"));
        }
        let edges =
            user["videos"]["edges"]
                .as_array()
                .ok_or(Box::<dyn std::error::Error>::from(
                    "Twitch API was expected to return an array and didn't return one",
                ))?;

        let mut cursor: Option<String> = None;
        let vods: Vec<Vod> = edges
            .into_iter()
            .filter_map(|edge| {
                if let Some(cur) = edge["cursor"].as_str() {
                    cursor = Some(cur.into());
                }

                let node: Option<Vod> = serde_json::from_value(edge["node"].clone()).ok();
                node.map(|mut vod| {
                    vod.url = format!("https://www.twitch.tv/videos/{}", vod.id);
                    vod
                })
            })
            .collect();
        Ok(Vods { vods, cursor })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Comment {
    created_at: String,
    updated_at: String,
    channel_id: String,
    content_id: String,
    content_offset_seconds: f64,
    message: String,
    user: CommentUser,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommentUser {
    display_name: String,
    id: Option<String>,
    username: String,
    biography: Option<String>,
    created_at: String,
    updated_at: String,
    profile_picture_url: String,
    color: Option<String>,
    badges: Vec<CommentUserBadge>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommentUserBadge {
    id: String,
    version: String,
}

#[derive(Debug)]
pub struct Comments {
    pub comments: Vec<Comment>,
    pub cursor: Option<String>,
}

impl Comments {
    pub async fn new(
        vodid: &str,
        cursor: Option<&str>,
    ) -> Result<Comments, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!(
                "https://api.twitch.tv/v5/videos/{}/comments?client_id={}&cursor={}",
                vodid,
                CLIENT_ID,
                cursor.unwrap_or("")
            ))
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let cursor = resp["_next"].as_str().map(|x| x.to_string());
        let comments = resp["comments"]
            .as_array()
            .ok_or(Box::<dyn std::error::Error>::from(
                "Twitch API was expected to return an array and didn't return one",
            ))?
            .into_iter()
            .filter_map(|comment| {
                Some(Comment {
                    created_at: comment["created_at"].as_str()?.to_string(),
                    updated_at: comment["updated_at"].as_str()?.to_string(),
                    channel_id: comment["channel_id"].as_str()?.to_string(),
                    content_id: comment["content_id"].as_str()?.to_string(),
                    content_offset_seconds: comment["content_offset_seconds"].as_f64()?,
                    message: comment["message"]["body"].as_str()?.to_string(),
                    user: CommentUser {
                        color: comment["message"]["user_color"]
                            .as_str()
                            .map(|x| x.to_string()),
                        display_name: comment["commenter"]["display_name"].as_str()?.to_string(),
                        id: comment["message"]["_id"].as_str().map(|x| x.to_string()),
                        username: comment["commenter"]["name"].as_str()?.to_string(),
                        created_at: comment["commenter"]["created_at"].as_str()?.to_string(),
                        updated_at: comment["commenter"]["updated_at"].as_str()?.to_string(),
                        profile_picture_url: comment["commenter"]["logo"].as_str()?.to_string(),
                        biography: comment["commenter"]["bio"].as_str().map(|x| x.to_string()),
                        badges: comment["message"]["user_badges"]
                            .as_array()
                            .map(|badges| {
                                badges
                                    .into_iter()
                                    .filter_map(|badge| {
                                        Some(CommentUserBadge {
                                            id: badge["_id"].as_str()?.to_string(),
                                            version: badge["version"].as_str()?.to_string(),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or(Vec::new()),
                    },
                })
            })
            .collect::<Vec<Comment>>();
        Ok(Comments { cursor, comments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn user_data() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let user_data = runtime.block_on(UserData::new("dariomocciatwitch"));
        match user_data {
            Ok(data) => println!("{:#?}", data),
            Err(_) => {
                user_data.unwrap();
            }
        }
    }

    #[test]
    fn vod_direct_link() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let vod_direct_link = runtime.block_on(Vod::fetch_direct_link("596966295"));
        match vod_direct_link {
            Ok(dlink) => println!("{}", dlink),
            Err(_) => {
                vod_direct_link.unwrap();
            }
        }
    }

    #[test]
    fn clips() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let clips = runtime.block_on(Clips::new(
            "dariomocciatwitch",
            "2020-04-18T19:35:06Z",
            "2020-04-22T19:35:06Z",
            None,
        ));
        match clips {
            Ok(c) => println!("{:#?}", c),
            Err(_) => {
                clips.unwrap();
            }
        }
    }

    #[test]
    fn vods() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let vods = runtime.block_on(Vods::new("dariomocciatwitch", None));
        match vods {
            Ok(v) => println!("{:#?}", v),
            Err(_) => {
                vods.unwrap();
            }
        }
    }

    #[test]
    fn comments() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let comments = runtime.block_on(Comments::new("596966295", None));
        match comments {
            Ok(c) => println!("{:#?}", c),
            Err(_) => {
                comments.unwrap();
            }
        }
    }

    #[test]
    fn m3u8_gen() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let m3u8 = runtime.block_on(Vod::m3u8_gen("596966295", Some(20.1), Some(140.3)));
        match m3u8 {
            Ok(m) => println!("{:#?}", m),
            Err(_) => {
                m3u8.unwrap();
            }
        }
    }
}
