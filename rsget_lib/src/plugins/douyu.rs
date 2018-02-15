use Streamable;
use reqwest;
use std::time::{SystemTime, UNIX_EPOCH};
use regex::Regex;

use utils::downloaders::flv_download;
use chrono::prelude::*;

use tokio_core::reactor::Core;
use hyper::header::{Headers, UserAgent};

use std;
use md5;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct DouyuGgad {
    play4: String,
    play1: String,
    videop: String,
    play2: String,
    play5: String,
    play3: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct DouyuServer {
    ip: String,
    port: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct DouyuCdn {
    name: String,
    cdn: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct DouyuP2p {
    player: usize,
    use_p2p: usize,
    w_dm: usize,
    m_dm: usize,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct DouyuGift {
    stay_time: usize,
    pdhimg: String,
    gx: usize,
    mobile_big_effect_icon_0: String,
    mgif: String,
    cimg: String,
    mobile_big_effect_icon_1: String,
    big_efect_icon: String,
    pdbimg: String,
    mobile_stay_time: String,
    mimg: String,
    brgb: String,
    mobile_big_effect_icon_3: String,
    m_ef_gif_2: String,
    pimg: String,
    pt: String,
    id: String,
    intro: String,
    pc: String,
    m_ef_gif_1: String,
    urgb: String,
    ef: usize,
    sort: String,
    mobile_big_effect_icon_2: String,
    ch: String,
    effect: String,
    himg: String,
    #[serde(rename = "type")]
    adtype: String,
    gt: String,
    mobile_small_effect_icon: String,
    grgb: String,
    drgb: String,
    pad_big_effect_icon: String,
    desc: String,
    mobimg: String,
    small_effect_icon: String,
    name: String,
    mobile_icon_v2: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct DouyuMultiBR {
    middle: String,
    middle2: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct DouyuData {
    use_p2p: String,
    show_details: String,
    nickname: String,
    ggad: DouyuGgad,
    anchor_city: String,
    specific_status: String,
    url: String,
    Servers: Vec<DouyuServer>,
    rtmp_cdn: String,
    specific_catalog: String,
    cate_id1: usize,
    show_status: String,
    game_icon_url: String,
    game_name: String,
    cdnsWithName: Vec<DouyuCdn>,
    p2p_settings: DouyuP2p,
    show_time: String,
    isVertical: usize,
    rtmp_live: String,
    fans: String,
    game_url: String,
    room_src: String,
    is_white_list: String,
    room_name: String,
    owner_uid: String,
    owner_avatar: String,
    black: Vec<usize>, // Not sure about this one,
    vertical_src: String,
    room_dm_delay: usize,
    owner_weight: String,
    is_pass_player: usize,
    hls_url: String,
    room_id: usize,
    cur_credit: String,
    gift_ver: String,
    low_credit: String,
    gift: Vec<DouyuGift>,
    rtmp_multi_bitrate: DouyuMultiBR,
    cdns: Vec<String>,
    online: usize,
    credit_illegal: String,
    vod_quality: String,
    cate_id: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct DouyuRoom {
    error: usize,
    data: DouyuData,
}


pub struct Douyu {
    data: DouyuRoom,
    room_id: u32,
}

impl Streamable for Douyu {
    fn new(url: String) -> Douyu {
        let room_id_re = Regex::new(r"com/([a-zA-Z0-9]+)").unwrap();
        let cap = room_id_re.captures(&url).unwrap();
        
        let client = reqwest::Client::new();
        let mut headers = Headers::new();
        headers.set(UserAgent::new("Mozilla/5.0 (compatible; MSIE 10.0; \
                                    Windows Phone 8.0; Trident/6.0; IEMobile/10.0; \
                                    ARM; Touch; NOKIA; Lumia 920)"));

        let room_id = match cap[1].parse::<u32>() {
            Ok(rid) => rid,
            Err(_) => {
                let html = match client.get(&url).send() {
                    Ok(mut res) => res.text().unwrap(),
                    Err(why) => {
                        info!("Failed getting url");
                        debug!("Failed getting url because of {}", why);
                        std::process::exit(1)
                    },
                };
                let re_room_id = Regex::new(r#""room_id" *:([0-9]+),"#).unwrap();
                let cap = re_room_id.captures(&html).unwrap();
                cap[1].parse::<u32>().unwrap()
            }
        };
        
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let ts = since_the_epoch.as_secs();

        let suffix =
            format!("room/{}?aid=wp&cdn={}&client_sys=wp&time={}", &room_id, "ws", ts);
 
        let api_secret = "zNzMV1y4EMxOHS6I5WKm".as_bytes();
        let mut hasher: md5::Context = md5::Context::new();

        hasher.consume(&suffix.as_bytes());
        hasher.consume(api_secret);

        let sign = format!("{:x}", hasher.compute());

        let json_url = format!("https://capi.douyucdn.cn/api/v1/{}&auth={}", &suffix, &sign);
        
        let mut resp = match client
            .get(&json_url)
            .headers(headers.clone())
            .send() {
                Ok(res) => res,
                Err(why) => {
                    info!("1 Error when getting site info ({})", why);
                    std::process::exit(1)
                }
            };
        
        let jres: Result<DouyuRoom, reqwest::Error> = resp.json();
        match jres {
            Ok(jre) => {
                Douyu {
                    data: jre,
                    room_id: room_id,
                }
                
            },
            Err(why) => {
                info!("Error when deserailising ({})", why);
                std::process::exit(1)
            }
        }
    }

    fn get_title(&self) -> Option<String> {
        //Some(self.panda_tv_room.data.roominfo.name.clone())
        None
    }

    fn get_author(&self) -> Option<String> {
        Some(self.data.data.nickname.clone())
    }

    fn is_online(&self) -> bool {
        self.data.data.online != 0
    }
    
    fn get_stream(&self) -> String {
        self.data.data.rtmp_live.clone()
    }

    fn get_ext(&self) -> String {
        String::from("flv")
    }
    
    fn get_default_name(&self) -> String {
        let local: DateTime<Local> = Local::now();
        format!("{}-{:04}-{:02}-{:02}-{:02}-{:02}-{}-{}.{}",
                self.room_id,
                local.year(),
                local.month(),
                local.day(),
                local.hour(),
                local.minute(),
                self.get_author().unwrap(),
                self.get_title().unwrap_or(String::from("TEST")),
                self.get_ext())
    }
    
    fn download(&self, core: &mut Core, path: String) -> Option<()> {
        if !self.is_online() {
            None
        } else {
            let local: DateTime<Local> = Local::now();
            println!("{} by {} ({}) <{:04}-{:02}-{:02}-{:02}-{:02}>",
                     self.get_title().unwrap(),
                     self.get_author().unwrap(),
                     self.room_id,
                     local.year(),
                     local.month(),
                     local.day(),
                     local.hour(),
                     local.minute(),
            );
            flv_download(core, self.get_stream(), path)
        }

    }
}
