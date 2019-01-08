use std::{
    collections::{
        HashSet,
        VecDeque,
    },
    io::{
        copy,
        BufWriter,
        Write,
    },
    sync::Arc,
    thread,
    time::{
        self,
        Duration,
    },
};

#[cfg(features = "spinner")]
use indicatif::{
    ProgressBar,
    ProgressStyle,
};

use reqwest::{
    Client as ReqwestClient,
    Request,
};

use hls_m3u8::MediaPlaylistOptions;

use parking_lot::{Mutex, RwLock};

use crate::error::Error;

const WRITE_SIZE: usize = 131_072;
const HLS_MAX_RETRIES: usize = 12;

/// A Enum with the types of streams supported
#[derive(Debug)]
pub enum StreamType {
    /// A stream that is just a chunked http response.
    Chuncked(Request),
    /// A m3u8 playlist, which may be a stream.
    HLS(Request),
}

#[derive(Debug, Clone)]
enum _StreamType {
    Chuncked,
    HLS,
}

#[derive(Debug)]
pub struct Stream {
    request: Request,
    stream_type: _StreamType,
    #[allow(dead_code)]
    spinner: bool,
}


impl Stream {
    /// Creates a new stream handler.
    ///
    /// # Note:
    ///
    /// For hls the headers are carried over to every subsequent call but the body
    /// is not.
    pub fn new(request: StreamType) -> Self {
        match request {
            StreamType::HLS(req) => Stream {
                request: req,
                stream_type: _StreamType::HLS,
                spinner: true,
            },
            StreamType::Chuncked(req) => Stream {
                request: req,
                stream_type: _StreamType::Chuncked,
                spinner: true,
            },
        }
    }

    /// Writes the stream to a file.
    pub fn write_file<W>(self, client: &ReqwestClient, writer: W) -> Result<u64, Error>
    where
        W: Write
    {
        match self.stream_type {
            _StreamType::Chuncked => Ok(self.chunked(client, writer)?),
            _StreamType::HLS => Ok(self.hls(client, writer)?),
        }
    }

    fn chunked<W>(self, client: &ReqwestClient, writer: W) -> Result<u64, Error>
    where
        W: Write
    {
        #[cfg(features = "spinner")]
        let spinner = ProgressBar::new(0);
        #[cfg(features = "spinner")]
        spinner.set_style(ProgressStyle::default_bar()
                          .template("{spinner:.green} [{elapsed_precise}] Streamed {bytes}")
                          .tick_chars("⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈ "));
        
        let mut buf_writer = BufWriter::with_capacity(WRITE_SIZE, writer);
        let mut source = client.execute(self.request)?;

        #[cfg(features = "spinner")]
        let size = copy(&mut spinner.wrap_read(source), &mut buf_writer);
        #[cfg(not(features = "spinner"))]
        let size = copy(&mut source, &mut buf_writer);
        Ok(size?)
    }

    // This currently clones the client to get a client to run the inner calls as well.
    fn hls<W>(self, client: &ReqwestClient, writer: W) -> Result<u64, Error>
    where
        W: Write
    {
        #[derive(Clone)]
        enum Hls {
            Url(String),
            StreamOver,
        }

        let to_work = Arc::new(Mutex::new(VecDeque::<Hls>::new())); // User in Inner
        let other_work = to_work.clone(); // Used in Outer
        let links: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new())); // Used in Inner
        let _outer_links = links.clone(); // Used in Outer (Not currently in use)

        // Inner loop -- Start
        // Here the handling of the m3u8 file is happening
        // it pushes it through the `to_work` mutex
        let inner_url = self.request.url().clone();
        let inner_headers = self.request.headers().clone();
        let headers = inner_headers.clone();
        let master_url = self.request.url().clone().join(".")?;
        let inner_client = client.to_owned();
        thread::spawn(move || {
            let mut counter = 0;

            loop {
                if counter > HLS_MAX_RETRIES {
                    // There have either been errors or no new segments
                    // for `HLS_MAX_RETRIES` times the segment duration given
                    // in the m3u8 playlist file.
                    let work_queue = &mut to_work.lock();
                    work_queue.push_back(Hls::StreamOver);
                    break;
                }

                // Use the same headers as the original request
                let mut res = match inner_client
                    .get(inner_url.clone())
                    .headers(inner_headers.clone())
                    .send()
                {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("[HLS] Playlist download failed!\n{}", e);
                        counter += 1;
                        continue;
                    }
                };

                let m3u8_string = match res.text() {
                    Ok(t) => t,
                    Err(e) => {
                        warn!("[HLS] Playlist text failed!\n{}", e);
                        counter += 1;
                        continue;
                    }
                };

                // Allow excess segment duration because a lot of video sites have
                // not very high quality m3u8 playlists, where the video segments,
                // may be longer than what the file specifies as max.
                let m3u8 = match MediaPlaylistOptions::new()
                    .allowable_excess_segment_duration(Duration::from_secs(10))
                    .parse(&m3u8_string)
                {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("[HLS] Parsing failed!\n{}", e);
                        trace!("[HLS]\n{}", &m3u8_string);
                        counter += 1;
                        continue;
                    }
                };

                // Get the target duration of a segment
                let target_duration = m3u8.target_duration_tag().duration();

                // Makes a iterator with the url parts from the playlist
                let m3u8_iterator = m3u8.segments().iter().map(|e| String::from(e.uri().trim()));

                for e in m3u8_iterator {
                    trace!("[HLS] Tries to inserts: {}", e);
                    // Check if we have the segment in our set already
                    if links.write().insert(e.clone()) {
                        // Reset the counter as we got a new segment.
                        counter = 0;

                        // Construct a url from the master and the segment.
                        let url_formatted = format!("{}{}", master_url.as_str(), &e.clone());
                        let work_queue = &mut to_work.lock();

                        // Check if the segment is a Afreeca preloading segment.
                        if !(e.contains("preloading")) {
                            info!("[HLS] Adds {}!", url_formatted);
                            // Add the segment to the queue.
                            work_queue.push_back(Hls::Url(url_formatted));
                        }
                    }
                }
                warn!("[HLS] Sleeps for {:#?}", target_duration);
                // Sleeps for the target duration.
                thread::sleep(target_duration);
                counter += 1;
            }
        });

        let mut total_size = 0;

        #[cfg(features = "spinner")]
        let spinner = ProgressBar::new(0);
        #[cfg(features = "spinner")]
        spinner.set_style(ProgressStyle::default_bar()
                          .template("{spinner:.green} [{elapsed_precise}] {bytes} Segments")
                          .tick_chars("⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈ "));
        #[cfg(features = "spinner")]
        spinner.enable_steady_tick(100);

        let mut buf_writer = BufWriter::with_capacity(WRITE_SIZE, writer);

        loop {
            let to_download = other_work.lock().pop_front();
            match to_download {
                Some(Hls::Url(u)) => {
                    let req = client.get(&u).headers(headers.clone()).build()?;
                    let size = download_to_file(client, req, &mut buf_writer)?;
                    #[cfg(features = "spinner")]
                    spinner.inc(size);
                    total_size += size;
                }
                Some(Hls::StreamOver) => break,
                None => {
                    trace!("[HLS] None to download!");
                    thread::sleep(time::Duration::from_secs(5));
                }
            }
        }
        Ok(total_size)
    }
}

#[inline]
fn download_to_file<W>(client: &ReqwestClient, request: Request, mut file: &mut BufWriter<W>) -> Result<u64, Error>
where
    W: Write
{
    let mut source = client.execute(request)?;
    let size = copy(&mut source, &mut file)?;
    Ok(size)
}
