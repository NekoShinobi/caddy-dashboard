use flate2::read::GzDecoder;
use std::{fs, io, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

fn year_month_from_unix(secs: u64) -> (u32, u32) {
    let mut days = (secs / 86400) as u32;
    let mut year = 1970u32;
    loop {
        let diy = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 366 } else { 365 };
        if days < diy { break; }
        days -= diy;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [31u32, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for &d in &month_days {
        if days < d { break; }
        days -= d;
        month += 1;
    }
    (year, month)
}

fn try_download(url: &str, dest: &PathBuf) -> bool {
    println!("cargo:warning=GeoIP: downloading {url}");
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(180))
        .build();
    let resp: ureq::Response = match agent.get(url).call() {
        Ok(r) => r,
        Err(e) => {
            println!("cargo:warning=GeoIP: download failed: {e}");
            return false;
        }
    };
    let mut gz = GzDecoder::new(resp.into_reader());
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    let mut f = match fs::File::create(dest) {
        Ok(f) => f,
        Err(e) => { println!("cargo:warning=GeoIP: write failed: {e}"); return false; }
    };
    match io::copy(&mut gz, &mut f) {
        Ok(_) => true,
        Err(e) => {
            println!("cargo:warning=GeoIP: decompress failed: {e}");
            let _ = fs::remove_file(dest);
            false
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=SKIP_DBIP_DOWNLOAD");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("dbip-city-lite.mmdb");

    if std::env::var("SKIP_DBIP_DOWNLOAD").as_deref() == Ok("1") {
        if !dest.exists() {
            fs::write(&dest, b"").unwrap();
        }
        return;
    }

    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let (year, month) = year_month_from_unix(secs);

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let cache_dir = manifest_dir.join(".dbip-cache");

    // Try current month, then previous month
    for (y, m) in [(year, month), if month == 1 { (year - 1, 12) } else { (year, month - 1) }] {
        let name = format!("dbip-city-lite-{y:04}-{m:02}.mmdb");
        let cached = cache_dir.join(&name);

        if cached.exists() {
            println!("cargo:warning=GeoIP: using cached {name}");
            fs::copy(&cached, &dest).unwrap();
            return;
        }

        let url = format!("https://download.db-ip.com/free/dbip-city-lite-{y:04}-{m:02}.mmdb.gz");
        if try_download(&url, &cached) {
            fs::copy(&cached, &dest).unwrap();
            println!("cargo:warning=GeoIP: cached as {name}");
            return;
        }
    }

    println!("cargo:warning=GeoIP: all downloads failed, heatmap unavailable");
    fs::write(&dest, b"").unwrap();
}
