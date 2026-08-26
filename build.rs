use std::{
    fs::{self, File},
    io,
    path::Path,
    thread,
};

const URLS: [&str; 25] = [
    "https://static.wikia.nocookie.net/rocketleague/images/0/00/Unranked_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/6/6c/Bronze1_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/5/5d/Bronze2_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/7/7a/Bronze3_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/a/a7/Champion1_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/0/07/Champion2_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/d/d9/Champion3_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/1/1d/Diamond1_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/b/b6/Diamond2_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/7/7a/Diamond3_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/8/8e/Gold1_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/b/be/Gold2_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/b/b1/Gold3_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/c/c5/Grand_champion_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/d/d4/Grand_champion1_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/6/6a/Grand_champion2_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/0/0c/Grand_champion3_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/7/77/Platinum1_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/e/e4/Platinum2_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/7/78/Platinum3_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/d/d5/Silver1_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/f/f8/Silver2_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/7/7c/Silver3_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/2/2d/Supersonic_Legend_rank_icon.png",
    "https://static.wikia.nocookie.net/rocketleague/images/a/ac/Avatar_icon.png",
];

// all the "png" URLs are actually webps btw

fn download(url: &str, out_dir: &Path) {
    let filename = Path::new(url.split('/').next_back().unwrap()).with_extension("webp");
    let path = out_dir.join(filename);
    if path.exists() {
        return;
    }

    let response = match ureq::get(url).call() {
        Ok(r) => r,
        Err(e) => {
            println!("cargo::warning=failed to fetch asset {url}: {e}");
            return;
        }
    };

    let mut file = File::create(path).unwrap();
    io::copy(&mut response.into_body().into_reader(), &mut file).unwrap();
}

fn main() {
    let out_dir = Path::new("assets/");
    fs::create_dir_all(out_dir).expect("couldnt create asset directory for images");

    thread::scope(|s| {
        for url in URLS {
            s.spawn(|| {
                download(url, out_dir);
            });
        }
    });
}
