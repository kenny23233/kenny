// src-tauri/tests/integration_test.rs
// 集成测试 (Tauri 2 标准位置: src-tauri/tests/<name>.rs)
// 跑法: cargo test --test integration_test -- --nocapture
//
// 范围: 跨模块、跨 crate type 的协作行为
//  - ytdlp::extract_domain 的 URL 边界
//  - ytdlp::ytdlp_path / ffmpeg_path 在开发模式下的解析
//  - Database 的 SQLite CRUD (用临时文件隔离真实数据)
//
// 注意: 不要重复 backend-pro 在 db.rs / ytdlp/mod.rs 内部的 #[cfg(test)] 单元测试。
// 本文件只测"集成视角", 即 lib crate 暴露的 pub API 的端到端行为。
// lib.rs 中 db/types/ytdlp 已被加 `pub` 以便 integration test 引用 (见 src/lib.rs:5-8)。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use video_toolbox_lib::db::Database;
use video_toolbox_lib::ytdlp::{extract_domain, ffmpeg_path, ytdlp_path};

// ---------- helpers ----------

/// 生成一个唯一的临时 db 路径, 测试可自选清理。
/// 用进程 ID + atomic 计数器 + 纳秒时间戳避免并行测试冲突。
fn unique_temp_db(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let file = format!("video_toolbox_it_{}_{}_{}_{}.db", label, pid, now, n);
    std::env::temp_dir().join(file)
}

fn cleanup(p: &PathBuf) {
    let _ = std::fs::remove_file(p);
}

// ============== 1) extract_domain 边界 ==============

#[test]
fn it_dom_01_basic_www_strip() {
    // 来自 ROADMAP 描述: youtube.com (去掉 www.)
    assert_eq!(
        extract_domain("https://www.youtube.com/watch?v=xxx"),
        Some("youtube.com".to_string())
    );
}

#[test]
fn it_dom_02_with_port() {
    // 带端口: 当前实现能正确处理 (split(':').next() 在 split('/').next() 之后)
    assert_eq!(
        extract_domain("https://example.com:8080/path"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("http://localhost:3000/api"),
        Some("localhost".to_string())
    );
}

#[test]
fn it_dom_03_with_query() {
    // query 字符串不应影响 host
    assert_eq!(
        extract_domain("https://example.com/?q=1&b=2"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("https://example.com/path?token=abc#frag"),
        Some("example.com".to_string())
    );
}

#[test]
fn it_dom_04_with_fragment_only() {
    assert_eq!(
        extract_domain("https://example.com/#anchor"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("https://example.com/path#anchor"),
        Some("example.com".to_string())
    );
}

#[test]
fn it_dom_05_userinfo_correctly_parsed() {
    // 修复 (P2): 改用 url::Url::parse 后, user:pass@host 能正确识别 host,
    // 之前手写 split 会把 "user" 误当 host。
    assert_eq!(
        extract_domain("https://user:pass@example.com/"),
        Some("example.com".to_string()),
        "user:pass@host 应当正确解析为 host, 不是 user"
    );
}

#[test]
fn it_dom_06_no_scheme() {
    // 无 scheme 直接返回 None
    assert_eq!(extract_domain("example.com/path"), None);
    assert_eq!(extract_domain("not a url"), None);
    assert_eq!(extract_domain(""), None);
}

#[test]
fn it_dom_07_bilibili_no_www() {
    // 国内主要源, 不带 www
    assert_eq!(
        extract_domain("https://www.bilibili.com/video/BV1xx"),
        Some("bilibili.com".to_string())
    );
    assert_eq!(
        extract_domain("https://bilibili.com/video/BV1xx"),
        Some("bilibili.com".to_string())
    );
}

#[test]
fn it_dom_08_subdomain_not_stripped() {
    // 子域名不应被 strip, 只 strip 顶级 www.
    // 例: m.youtube.com -> "m.youtube.com" (不是 youtube.com)
    assert_eq!(
        extract_domain("https://m.youtube.com/feed"),
        Some("m.youtube.com".to_string())
    );
    assert_eq!(
        extract_domain("https://v.douyin.com/iJ5n6Q7x/"),
        Some("v.douyin.com".to_string())
    );
}

// ============== 2) ytdlp_path / ffmpeg_path 查找 ==============

#[test]
fn it_path_01_ytdlp_path_resolves_in_dev_mode() {
    // 开发模式 (cargo test): CARGO_MANIFEST_DIR 一定存在, 应当回退到 bin/yt-dlp.exe
    let p = ytdlp_path();
    assert!(
        p.ends_with("yt-dlp.exe"),
        "ytdlp_path 应以 yt-dlp.exe 结尾, 实际: {:?}",
        p
    );
    assert!(
        p.exists(),
        "ytdlp_path 应当指向真实文件, 实际: {:?}",
        p
    );
}

#[test]
fn it_path_02_ffmpeg_path_some_in_dev_mode() {
    // 同上, ffmpeg_path 在 dev 模式应当解析到 bin/ffmpeg.exe
    let p = ffmpeg_path();
    assert!(p.is_some(), "ffmpeg_path 应当返回 Some(ffmpeg.exe 路径)");
    let p = p.unwrap();
    assert!(
        p.ends_with("ffmpeg.exe"),
        "ffmpeg_path 应以 ffmpeg.exe 结尾, 实际: {:?}",
        p
    );
    assert!(p.exists(), "ffmpeg.exe 实际存在, 路径: {:?}", p);
}

#[test]
fn it_path_03_ytdlp_path_is_absolute() {
    // 安全性: 路径必须是绝对路径, 否则 yt-dlp spawn 时会找不到
    let p = ytdlp_path();
    assert!(p.is_absolute(), "ytdlp_path 应当是绝对路径, 实际: {:?}", p);
}

#[test]
fn it_path_04_ffmpeg_path_is_absolute() {
    let p = ffmpeg_path();
    if let Some(p) = p {
        assert!(p.is_absolute(), "ffmpeg_path 应当是绝对路径, 实际: {:?}", p);
    }
}

#[test]
fn it_path_05_ytdlp_and_ffmpeg_in_same_bin_dir() {
    // 两个二进制应在同一目录, 便于部署
    let ytdlp = ytdlp_path();
    if let Some(ff) = ffmpeg_path() {
        assert_eq!(
            ytdlp.parent(),
            ff.parent(),
            "yt-dlp.exe 和 ffmpeg.exe 应当在同目录"
        );
    }
}

// ============== 3) Database 集成 (SQLite CRUD) ==============
// 注意: backend-pro 已经在 db.rs 内部 #[cfg(test)] 写了基础单测
// (test_add_and_list_history / test_list_history_pagination / test_list_history_search /
//  test_delete_history / test_clear_history / test_get_history_count_with_search /
//  test_setting_roundtrip)。
// 本节只测"集成视角" — 跨连接持久化、Unicode 兼容、boundary 行为。

#[test]
fn it_db_01_add_then_list_roundtrip() {
    let p = unique_temp_db("add_list");
    let db = Database::open(&p).expect("open db");

    let id1 = db
        .add_history("https://a.example.com/v1", "视频1", r"C:\tmp\a.mp4", Some(1234))
        .expect("add 1");
    let id2 = db
        .add_history("https://b.example.com/v2", "视频2", r"C:\tmp\b.mp4", None)
        .expect("add 2");

    assert!(id1 > 0 && id2 > 0, "id 必须 > 0, 实际: {} {}", id1, id2);
    assert_ne!(id1, id2, "两条记录的 id 必须不同");

    let list = db.list_history(10, 0, None).expect("list");
    assert_eq!(list.len(), 2);

    // 最近插入的 id2 应当在最前 (DESC 排序)
    assert_eq!(list[0].id, id2, "应按 id DESC 排序");
    assert_eq!(list[0].url, "https://b.example.com/v2");
    assert_eq!(list[0].title, "视频2");
    assert_eq!(list[0].save_path, r"C:\tmp\b.mp4");
    assert_eq!(list[0].size_bytes, None);

    assert_eq!(list[1].id, id1);
    assert_eq!(list[1].size_bytes, Some(1234));
    assert_eq!(list[1].status, "completed");

    // 时间戳应当是 ISO8601 字符串, 含 'T' 分隔
    assert!(
        list[0].downloaded_at.contains('T'),
        "downloaded_at 应当是 ISO8601, 实际: {}",
        list[0].downloaded_at
    );

    cleanup(&p);
}

#[test]
fn it_db_02_list_history_orders_by_id_desc() {
    let p = unique_temp_db("order");
    let db = Database::open(&p).unwrap();

    // 插入 5 条, 验证 DESC
    let mut ids = Vec::new();
    for i in 0..5 {
        let id = db
            .add_history(
                &format!("https://e.com/{}", i),
                &format!("t{}", i),
                "/tmp",
                None,
            )
            .unwrap();
        ids.push(id);
    }

    let list = db.list_history(100, 0, None).unwrap();
    assert_eq!(list.len(), 5);

    // 第一个应当是最后一个插入的 id
    assert_eq!(list[0].id, ids[4]);
    assert_eq!(list[4].id, ids[0]);

    cleanup(&p);
}

#[test]
fn it_db_03_delete_history_removes_entry() {
    let p = unique_temp_db("delete");
    let db = Database::open(&p).unwrap();

    let id = db
        .add_history("https://x", "x", "/tmp", None)
        .unwrap();
    assert_eq!(db.list_history(10, 0, None).unwrap().len(), 1);

    db.delete_history(id).unwrap();
    assert_eq!(
        db.list_history(10, 0, None).unwrap().len(),
        0,
        "delete 之后 list 应当为空"
    );

    cleanup(&p);
}

#[test]
fn it_db_04_delete_nonexistent_id_silent() {
    let p = unique_temp_db("delete_404");
    let db = Database::open(&p).unwrap();

    // 不存在的 id 应当静默成功 (不报错)
    let result = db.delete_history(99999);
    assert!(result.is_ok(), "删除不存在的 id 不应报错");

    // 列表保持空
    assert_eq!(db.list_history(10, 0, None).unwrap().len(), 0);

    cleanup(&p);
}

#[test]
fn it_db_05_settings_roundtrip() {
    let p = unique_temp_db("settings");
    let db = Database::open(&p).unwrap();

    // 不存在的 key 返回 None
    assert_eq!(db.get_setting("nope").unwrap(), None);

    // 写入 + 读取
    db.set_setting("default_save_dir", r"C:\Users\me\Downloads")
        .unwrap();
    let v = db.get_setting("default_save_dir").unwrap();
    assert_eq!(v, Some(r"C:\Users\me\Downloads".to_string()));

    cleanup(&p);
}

#[test]
fn it_db_06_settings_overwrite() {
    let p = unique_temp_db("settings_overwrite");
    let db = Database::open(&p).unwrap();

    db.set_setting("proxy", "http://1.1.1.1:8080").unwrap();
    assert_eq!(
        db.get_setting("proxy").unwrap(),
        Some("http://1.1.1.1:8080".to_string())
    );

    db.set_setting("proxy", "http://2.2.2.2:3128").unwrap();
    assert_eq!(
        db.get_setting("proxy").unwrap(),
        Some("http://2.2.2.2:3128".to_string()),
        "第二次写应当覆盖第一次"
    );

    cleanup(&p);
}

#[test]
fn it_db_07_settings_persists_across_connections() {
    // 模拟应用重启: 关闭 db, 用同一文件重新 open
    let p = unique_temp_db("persist");
    {
        let db = Database::open(&p).unwrap();
        db.set_setting("k1", "v1").unwrap();
        db.add_history("https://a", "a", "/tmp", Some(100)).unwrap();
    }
    {
        let db2 = Database::open(&p).unwrap();
        assert_eq!(db2.get_setting("k1").unwrap(), Some("v1".to_string()));
        let list = db2.list_history(10, 0, None).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].url, "https://a");
        assert_eq!(list[0].size_bytes, Some(100));
    }
    cleanup(&p);
}

#[test]
fn it_db_08_empty_list_on_fresh_db() {
    let p = unique_temp_db("empty");
    let db = Database::open(&p).unwrap();
    assert!(db.list_history(10, 0, None).unwrap().is_empty());
    assert_eq!(db.get_setting("anything").unwrap(), None);
    cleanup(&p);
}

#[test]
fn it_db_09_list_history_respects_limit_and_offset() {
    // 集成测试 backend-pro 已经在内部测过 pagination, 这里强调
    // "limit 必须 >= 1" 和 "offset 边界" 在我们的 wiring 中成立
    let p = unique_temp_db("limit");
    let db = Database::open(&p).unwrap();

    for i in 0..10 {
        db.add_history(&format!("u{}", i), "t", "/tmp", None).unwrap();
    }

    let limited = db.list_history(3, 0, None).unwrap();
    assert_eq!(limited.len(), 3, "limit=3 应当只返回 3 条");

    // offset=7 应当返回最后 3 条 (id 1, 2, 3)
    let tail = db.list_history(3, 7, None).unwrap();
    assert_eq!(tail.len(), 3);
    // DESC 排序: 末页第一条 = id=3
    assert_eq!(tail[0].id, 3);

    cleanup(&p);
}

#[test]
fn it_db_10_unicode_content_survives_roundtrip() {
    // 边界: 中文 / emoji 写入不能损坏
    let p = unique_temp_db("unicode");
    let db = Database::open(&p).unwrap();

    let url = "https://www.bilibili.com/video/BV1中文测试🎬";
    let title = "📹 视频标题 with 中文 & emoji 🎉";
    let save = r"C:\保存目录\视频\测试 中文.mp4";

    let id = db.add_history(url, title, save, Some(2048)).unwrap();
    let list = db.list_history(10, 0, None).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, id);
    assert_eq!(list[0].url, url);
    assert_eq!(list[0].title, title);
    assert_eq!(list[0].save_path, save);

    cleanup(&p);
}

#[test]
fn it_db_11_search_filters_by_url_or_title() {
    // 集成: search 参数穿透到 SQL
    let p = unique_temp_db("search");
    let db = Database::open(&p).unwrap();

    db.add_history("https://youtube.com/abc", "Cool video", "/p", None).unwrap();
    db.add_history("https://bilibili.com/xyz", "Some video", "/p", None).unwrap();
    db.add_history("https://example.org", "Unrelated", "/p", None).unwrap();

    let r1 = db.list_history(10, 0, Some("youtube")).unwrap();
    assert_eq!(r1.len(), 1, "搜索 'youtube' 应只命中 youtube 链接");
    assert_eq!(r1[0].url, "https://youtube.com/abc");

    let r2 = db.list_history(10, 0, Some("video")).unwrap();
    assert_eq!(r2.len(), 2, "搜索 'video' 应命中 title 含 video 的两条");

    let r3 = db.list_history(10, 0, Some("nope")).unwrap();
    assert_eq!(r3.len(), 0, "搜索无匹配应返回空");

    // 空字符串视作无 search
    let r4 = db.list_history(10, 0, Some("")).unwrap();
    assert_eq!(r4.len(), 3);

    cleanup(&p);
}

#[test]
fn it_db_12_count_consistent_with_list() {
    // get_history_count 应当与 list_history 长度一致
    let p = unique_temp_db("count");
    let db = Database::open(&p).unwrap();

    for i in 0..7 {
        db.add_history(&format!("https://e.com/{}", i), "t", "/p", None).unwrap();
    }

    let count = db.get_history_count(None).unwrap();
    let list = db.list_history(100, 0, None).unwrap();
    assert_eq!(count as usize, list.len(), "count 与 list 长度应一致");

    cleanup(&p);
}
