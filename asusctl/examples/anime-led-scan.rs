//! LED scanning tool for discovering AniMe Matrix buffer-to-LED mappings.
//!
//! This tool lights up one buffer index at a time, allowing you to observe
//! which physical LED corresponds to each buffer position. This is essential
//! for mapping new device types like G835L where the exact layout is unknown.
//!
//! You might want to use it slowly, as it sometimes doesn't work properly.
//! Maybe there's better ways to make this reliable but for now it works for my use case.
//!
//! # Usage
//! ```
//! cargo run --example anime-led-scan -- [options]
//! ```
//!
//! # Controls
//! - `n` or `Enter`: Next index
//! - `p` or `Backspace`: Previous index
//! - `j` followed by number: Jump to specific index
//! - `+` / `-`: Adjust step size (default 1)
//! - `s`: Save current index to notes file
//! - `r`: Mark current index as row start
//! - `q` or `Ctrl+C`: Quit
//!
//! # Output
//! Creates a `led-scan-notes.txt` file with recorded observations.

use std::env;
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};

use rog_anime::usb::{Brightness, get_anime_type};
use rog_anime::{AnimeDataBuffer, AnimeType};
use rog_dbus::zbus_anime::AnimeProxyBlocking;
use zbus::blocking::Connection;

/// Saved device state for restoration on exit
struct SavedState {
    builtins_enabled: bool,
    brightness: Brightness,
    display_enabled: bool,
}

fn print_help(scan_len: usize, buffer_len: usize) {
    println!("\n=== LED Scan Tool ===");
    println!(
        "Scan range: 0-{} (buffer size: {})",
        scan_len - 1,
        buffer_len
    );
    println!("Commands:");
    println!("  n, Enter     - Next index");
    println!("  p, Backspace - Previous index");
    println!("  j <num>      - Jump to index");
    println!("  + / -        - Increase/decrease step size");
    println!("  s            - Save note for current index");
    println!("  r            - Mark as row start");
    println!("  a            - Auto-scan (runs through all indices)");
    println!("  f            - Fill all buffer bytes");
    println!("  f <start> <end> - Fill range (inclusive)");
    println!("  p1/p2/p3     - Fill pane 1/2/3 only (each is 627 bytes)");
    println!("  hold         - Hold current LED (press Enter to release)");
    println!("  hold <s> <e> - Hold range (press Enter to release)");
    println!("  c            - Clear display");
    println!("  row          - Step through rows (G835L, provisional)");
    println!("  row <n>      - Show specific row (G835L, provisional)");
    println!("  allrows      - Light all rows sequentially (G835L)");
    println!("  rowmap       - Print the full row mapping (G835L)");
    println!("  h            - Show this help");
    println!("  q            - Quit and restore state");
    println!();
}

fn save_note(index: usize, note: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("led-scan-notes.txt")?;
    writeln!(file, "Index {}: {}", index, note)?;
    Ok(())
}

fn write_single_led(
    proxy: &AnimeProxyBlocking,
    anime_type: AnimeType,
    index: usize,
    brightness: u8,
) {
    let mut buffer = AnimeDataBuffer::new(anime_type);
    let data = buffer.data_mut();
    if index < data.len() {
        data[index] = brightness;
    }
    if let Err(e) = proxy.write(buffer) {
        eprintln!("Error writing to device: {}", e);
    }
}

fn clear_display(proxy: &AnimeProxyBlocking, anime_type: AnimeType) {
    let buffer = AnimeDataBuffer::new(anime_type);
    let _ = proxy.write(buffer);
}

fn fill_display(proxy: &AnimeProxyBlocking, anime_type: AnimeType, brightness: u8) {
    let mut buffer = AnimeDataBuffer::new(anime_type);
    let data = buffer.data_mut();
    for byte in data.iter_mut() {
        *byte = brightness;
    }
    if let Err(e) = proxy.write(buffer) {
        eprintln!("Error writing to device: {}", e);
    }
}

/// Fill a range of LEDs. Both start and end are INCLUSIVE.
fn fill_range(
    proxy: &AnimeProxyBlocking,
    anime_type: AnimeType,
    start: usize,
    end: usize,
    brightness: u8,
) {
    let mut buffer = AnimeDataBuffer::new(anime_type);
    let data = buffer.data_mut();
    for i in start..=end.min(data.len().saturating_sub(1)) {
        data[i] = brightness;
    }
    if let Err(e) = proxy.write(buffer) {
        eprintln!("Error writing to device: {}", e);
    }
}

fn fill_pane(proxy: &AnimeProxyBlocking, anime_type: AnimeType, pane: usize, brightness: u8) {
    const PANE_LEN: usize = 627;
    let start = pane * PANE_LEN;
    let end = start + PANE_LEN - 1;
    fill_range(proxy, anime_type, start, end, brightness);
}

/// G835L row pattern (PROVISIONAL - needs hardware verification):
/// - Rows 0-1: 1 LED each
/// - Rows 2-3: 2 LEDs each
/// - ... (pairs of rows with same length)
/// - Rows 26-27: 14 LEDs each
/// - Rows 28+: 15 LEDs each (constant)
///
/// Returns (start_index, end_index_inclusive, row_length)
fn g835l_row_bounds(row: usize) -> (usize, usize, usize) {
    let triangle_rows = 28;
    let triangle_leds = 210;

    if row < triangle_rows {
        let length = row / 2 + 1;
        let mut start = 0usize;
        for r in 0..row {
            start += r / 2 + 1;
        }
        (start, start + length - 1, length)
    } else {
        let rows_after_triangle = row - triangle_rows;
        let start = triangle_leds + rows_after_triangle * 15;
        (start, start + 14, 15)
    }
}

fn g835l_total_rows() -> usize {
    28 + 40
}

fn save_state(proxy: &AnimeProxyBlocking) -> SavedState {
    SavedState {
        builtins_enabled: proxy.builtins_enabled().unwrap_or(false),
        brightness: proxy.brightness().unwrap_or(Brightness::Med),
        display_enabled: proxy.enable_display().unwrap_or(true),
    }
}

fn restore_state(proxy: &AnimeProxyBlocking, state: &SavedState) {
    let _ = proxy.set_builtins_enabled(state.builtins_enabled);
    let _ = proxy.set_brightness(state.brightness);
    let _ = proxy.set_enable_display(state.display_enabled);
    let _ = proxy.run_main_loop(true);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut start_index = 0usize;
    let mut brightness = 200u8;
    let mut scan_limit: Option<usize> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--start" | "-s" => {
                if i + 1 < args.len() {
                    start_index = args[i + 1].parse().unwrap_or(0);
                    i += 1;
                }
            }
            "--brightness" | "-b" => {
                if i + 1 < args.len() {
                    brightness = args[i + 1].parse().unwrap_or(200);
                    i += 1;
                }
            }
            "--limit" | "-l" => {
                if i + 1 < args.len() {
                    scan_limit = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("LED Scan Tool for AniMe Matrix");
                println!();
                println!("Usage: anime-led-scan [options]");
                println!();
                println!("Options:");
                println!("  -s, --start <N>      Start at index N (default: 0)");
                println!("  -b, --brightness <N> LED brightness 0-255 (default: 200)");
                println!("  -l, --limit <N>      Cap scan range to N indices (e.g. 810 for G835L)");
                println!("  -h, --help           Show this help");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let conn = match Connection::system() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to D-Bus: {}", e);
            eprintln!("Make sure asusd is running.");
            return;
        }
    };

    let proxy = match AnimeProxyBlocking::new(&conn) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create Anime proxy: {}", e);
            eprintln!("Make sure asusd supports your device.");
            return;
        }
    };

    let anime_type = get_anime_type();
    let buffer_len = anime_type.data_length();
    let scan_len = scan_limit.unwrap_or(buffer_len).min(buffer_len);

    println!("=== LED Scan Tool ===");
    println!("Device type: {:?}", anime_type);
    println!("Buffer length: {} bytes", buffer_len);
    println!("Scan range: 0-{}", scan_len - 1);
    println!("Brightness: {}", brightness);
    println!();

    // Save current state for restoration
    let saved_state = save_state(&proxy);
    println!("Saved device state for restoration on exit.");

    // Stop system animations
    if let Err(e) = proxy.run_main_loop(false) {
        eprintln!("Warning: Could not stop main loop: {}", e);
    }
    println!("Stopped system animations.");

    print_help(scan_len, buffer_len);

    let mut current_index = start_index.min(scan_len - 1);
    let mut step = 1usize;

    write_single_led(&proxy, anime_type, current_index, brightness);
    println!(">>> Index: {} (step: {})", current_index, step);

    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        input.clear();
        print!("> ");
        io::stdout().flush().unwrap();

        if stdin.lock().read_line(&mut input).is_err() {
            break;
        }

        let cmd = input.trim();

        match cmd {
            "q" | "quit" | "exit" => {
                clear_display(&proxy, anime_type);
                restore_state(&proxy, &saved_state);
                println!("Restored device state. Goodbye!");
                break;
            }
            "n" | "" => {
                current_index = (current_index + step).min(scan_len - 1);
                write_single_led(&proxy, anime_type, current_index, brightness);
                println!(">>> Index: {} (step: {})", current_index, step);
            }
            "p" => {
                current_index = current_index.saturating_sub(step);
                write_single_led(&proxy, anime_type, current_index, brightness);
                println!(">>> Index: {} (step: {})", current_index, step);
            }
            "+" => {
                step = step.saturating_mul(2).max(1);
                println!("Step size: {}", step);
            }
            "-" => {
                step = step.saturating_div(2).max(1);
                println!("Step size: {}", step);
            }
            "r" => {
                if let Err(e) = save_note(current_index, "ROW START") {
                    eprintln!("Error saving note: {}", e);
                } else {
                    println!("Saved: Index {} marked as ROW START", current_index);
                }
            }
            "h" | "help" | "?" => {
                print_help(scan_len, buffer_len);
            }
            cmd if cmd.starts_with('j') => {
                let num_str = cmd.trim_start_matches('j').trim();
                if let Ok(idx) = num_str.parse::<usize>() {
                    if idx < scan_len {
                        current_index = idx;
                        write_single_led(&proxy, anime_type, current_index, brightness);
                        println!(">>> Index: {} (step: {})", current_index, step);
                    } else {
                        println!("Index {} out of range (max: {})", idx, scan_len - 1);
                    }
                } else {
                    println!("Usage: j <number>");
                }
            }
            cmd if cmd.starts_with('s') && !cmd.starts_with("show") => {
                let note = cmd.trim_start_matches('s').trim();
                let note = if note.is_empty() { "observed" } else { note };
                if let Err(e) = save_note(current_index, note) {
                    eprintln!("Error saving note: {}", e);
                } else {
                    println!("Saved note for index {}", current_index);
                }
            }
            "a" => {
                println!("Auto-scan mode (0 to {})...", scan_len - 1);
                let delay = std::time::Duration::from_millis(10);
                for idx in current_index..scan_len {
                    write_single_led(&proxy, anime_type, idx, brightness);
                    print!("\rIndex: {} / {}   ", idx, scan_len - 1);
                    io::stdout().flush().unwrap();
                    std::thread::sleep(delay);
                    current_index = idx;
                }
                println!();
                println!("Auto-scan complete. Current index: {}", current_index);
            }
            "c" => {
                clear_display(&proxy, anime_type);
                println!("Display cleared");
            }
            "f" => {
                fill_display(&proxy, anime_type, brightness);
                println!("All buffer bytes filled at brightness {}", brightness);
            }
            "p1" => {
                fill_pane(&proxy, anime_type, 0, brightness);
                println!("Pane 1 (indices 0-626) filled");
            }
            "p2" => {
                fill_pane(&proxy, anime_type, 1, brightness);
                println!("Pane 2 (indices 627-1253) filled");
            }
            "p3" => {
                fill_pane(&proxy, anime_type, 2, brightness);
                println!("Pane 3 (indices 1254-1880) filled");
            }
            cmd if cmd.starts_with("f ") => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() == 3 {
                    if let (Ok(start), Ok(end)) =
                        (parts[1].parse::<usize>(), parts[2].parse::<usize>())
                    {
                        fill_range(&proxy, anime_type, start, end, brightness);
                        println!("Filled indices {} to {}", start, end);
                    } else {
                        println!("Usage: f <start> <end>");
                    }
                } else {
                    println!("Usage: f <start> <end>");
                }
            }
            "show" => {
                write_single_led(&proxy, anime_type, current_index, brightness);
                println!(">>> Index: {} (step: {})", current_index, step);
            }
            "row" => {
                if anime_type != AnimeType::G835L {
                    println!(
                        "Warning: Row commands use G835L mapping (provisional). You can add to this code to support other types. `examples/anime-led-scan.rs[402:425]`"
                    );
                }
                println!("Row stepping mode. Press Enter for next row, 'q' to quit.");
                let total = g835l_total_rows();
                for row_num in 0..total {
                    let (start, end, len) = g835l_row_bounds(row_num);
                    if end >= scan_len {
                        println!("Row {} exceeds scan limit, stopping.", row_num);
                        break;
                    }
                    println!("Row {}: indices {}-{} ({} LEDs)", row_num, start, end, len);
                    fill_range(&proxy, anime_type, start, end, brightness);
                    input.clear();
                    print!("(Enter=next, q=quit) > ");
                    io::stdout().flush().unwrap();
                    if stdin.lock().read_line(&mut input).is_err() {
                        break;
                    }
                    if input.trim() == "q" {
                        break;
                    }
                    clear_display(&proxy, anime_type);
                }
                println!("Row stepping done.");
            }
            cmd if cmd.starts_with("row ") => {
                if anime_type != AnimeType::G835L {
                    println!("Warning: Row commands use G835L mapping (provisional).");
                }
                let row_str = cmd.trim_start_matches("row ").trim();
                if let Ok(row_num) = row_str.parse::<usize>() {
                    let total = g835l_total_rows();
                    if row_num < total {
                        let (start, end, len) = g835l_row_bounds(row_num);
                        if end < scan_len {
                            println!("Row {}: indices {}-{} ({} LEDs)", row_num, start, end, len);
                            fill_range(&proxy, anime_type, start, end, brightness);
                        } else {
                            println!("Row {} exceeds scan limit", row_num);
                        }
                    } else {
                        println!("Row {} out of range (max: {})", row_num, total - 1);
                    }
                } else {
                    println!("Usage: row <number>");
                }
            }
            "allrows" => {
                if anime_type != AnimeType::G835L {
                    println!("Warning: Row commands use G835L mapping (provisional).");
                }
                println!("Lighting all rows sequentially (200ms each)...");
                let total = g835l_total_rows();
                let delay = std::time::Duration::from_millis(200);
                for row_num in 0..total {
                    let (start, end, len) = g835l_row_bounds(row_num);
                    if end >= scan_len {
                        println!("\nRow {} exceeds scan limit, stopping.", row_num);
                        break;
                    }
                    print!(
                        "\rRow {}/{}: indices {}-{} ({} LEDs)    ",
                        row_num,
                        total - 1,
                        start,
                        end,
                        len
                    );
                    io::stdout().flush().unwrap();
                    fill_range(&proxy, anime_type, start, end, brightness);
                    std::thread::sleep(delay);
                    clear_display(&proxy, anime_type);
                }
                println!("\nDone.");
            }
            "rowmap" => {
                if anime_type != AnimeType::G835L {
                    println!("Warning: Row map is for G835L (provisional).");
                }
                println!("G835L Row Map:");
                let total = g835l_total_rows();
                for row_num in 0..total {
                    let (start, end, len) = g835l_row_bounds(row_num);
                    let marker = if end >= scan_len {
                        " (exceeds limit)"
                    } else {
                        ""
                    };
                    println!(
                        "  Row {:2}: indices {:4}-{:4} ({:2} LEDs){}",
                        row_num, start, end, len, marker
                    );
                }
            }
            "hold" => {
                // Single write, wait for Enter to release
                println!("Holding index {}. Press Enter to release...", current_index);
                write_single_led(&proxy, anime_type, current_index, brightness);
                input.clear();
                let _ = stdin.lock().read_line(&mut input);
                clear_display(&proxy, anime_type);
                println!("Released.");
            }
            cmd if cmd.starts_with("hold ") => {
                let arg = cmd.trim_start_matches("hold ").trim();
                let (start, end): (usize, usize) = match arg {
                    "p1" | "1" => (0, 626),
                    "p2" | "2" => (627, 1253),
                    _ => {
                        let parts: Vec<&str> = arg.split_whitespace().collect();
                        if parts.len() == 2 {
                            if let (Ok(s), Ok(e)) = (parts[0].parse(), parts[1].parse()) {
                                (s, e)
                            } else {
                                println!("Usage: hold p1, hold p2, or hold <start> <end>");
                                continue;
                            }
                        } else {
                            println!("Usage: hold p1, hold p2, or hold <start> <end>");
                            continue;
                        }
                    }
                };
                println!("Holding range {}-{}. Press Enter to release...", start, end);
                fill_range(&proxy, anime_type, start, end, brightness);
                input.clear();
                let _ = stdin.lock().read_line(&mut input);
                clear_display(&proxy, anime_type);
                println!("Released.");
            }
            _ => {
                if let Ok(idx) = cmd.parse::<usize>() {
                    if idx < scan_len {
                        current_index = idx;
                        write_single_led(&proxy, anime_type, current_index, brightness);
                        println!(">>> Index: {} (step: {})", current_index, step);
                    } else {
                        println!("Index {} out of range (max: {})", idx, scan_len - 1);
                    }
                } else {
                    println!("Unknown command: '{}'. Type 'h' for help.", cmd);
                }
            }
        }
    }
}
