use chrono::{Datelike, Duration, NaiveDate, Weekday};
use clap::Parser;
use std::collections::HashMap;

/// A program to compute the liturgical pericope and Bible readings for a given date.
/// It supports both default (placeholder) readings and custom Bible readings
/// keyed by (event label, set).
///
/// Example usage:
///   cargo run -- "08/02/2025"
///   cargo run -- "13/12/2026"
///   cargo run -- "19/12/2027"
///   cargo run -- "02/01/2028"
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Date in dd/mm/yyyy format, e.g. "08/02/2025"
    date: String,
}

/// An event in the liturgical calendar.
#[derive(Debug, Clone)]
struct Event {
    label: String,
    date: NaiveDate,
    altar_color: String,
    /// Priority is used when two events fall on the same day;
    /// higher priority events override lower ones.
    priority: u8,
}

/// Computes the First Sunday of Advent for a given year.
/// In this calendar, the first Advent Sunday is defined as the first Sunday on or after November 21.
fn first_sunday_of_advent(year: i32) -> NaiveDate {
    let candidate = NaiveDate::from_ymd(year, 11, 21);
    let offset = (7 - candidate.weekday().num_days_from_sunday()) % 7;
    candidate + Duration::days(offset as i64)
}

/// Returns the first Sunday on or after the given date.
fn first_sunday_on_or_after(mut date: NaiveDate) -> NaiveDate {
    while date.weekday() != Weekday::Sun {
        date = date + Duration::days(1);
    }
    date
}

/// Computes the date of Easter for the given year (using the Meeus/Jones/Butcher algorithm).
fn compute_easter(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31; // 3 = March, 4 = April
    let day = ((h + l - 7 * m + 114) % 31) + 1;
    NaiveDate::from_ymd(year, month as u32, day as u32)
}

/// Generates all events for the given liturgical year.
/// The liturgical year runs from the First Sunday of Advent of the given year
/// until (but not including) the First Sunday of Advent of the next year.
fn generate_events(lit_year: i32) -> Vec<Event> {
    let start = first_sunday_of_advent(lit_year);
    let end = first_sunday_of_advent(lit_year + 1);

    let mut events_map: HashMap<NaiveDate, Event> = HashMap::new();

    // Helper: insert an event if its date falls between [start, end).
    let mut insert_event = |ev: Event| {
        if ev.date >= start && ev.date < end {
            events_map
                .entry(ev.date)
                .and_modify(|existing| {
                    if ev.priority > existing.priority {
                        *existing = ev.clone();
                    }
                })
                .or_insert(ev);
        }
    };

    // 1. Advent series (5 Sundays, purple), priority = 1.
    for i in 0..=4 {
        let ev = Event {
            label: if i == 0 {
                "advent".to_string()
            } else {
                format!("advent + {}", i)
            },
            date: start + Duration::days(7 * i as i64),
            altar_color: "purple".to_string(),
            priority: 1,
        };
        insert_event(ev);
    }

    // 2. Christmas series (white), priority = 2.
    // "christmas" is fixed to December 25.
    // "christmas + 1" is the first Sunday on or after December 26.
    // A candidate "new year" event is computed as 7 days later.
    // If that candidate falls before January 6 of the following year, omit it so that
    // that date becomes the start of the Epiphany series.
    let christmas_fixed = NaiveDate::from_ymd(lit_year, 12, 25);
    let christmas_plus1 = first_sunday_on_or_after(christmas_fixed + Duration::days(1));
    let new_year_candidate = christmas_plus1 + Duration::days(7);
    let new_year_threshold = NaiveDate::from_ymd(lit_year + 1, 1, 6);
    let mut christmas_events = vec![
        ("christmas", christmas_fixed),
        ("christmas + 1", christmas_plus1),
    ];
    if new_year_candidate >= new_year_threshold {
        christmas_events.push(("new year", new_year_candidate));
    }
    for (label, date) in christmas_events {
        insert_event(Event {
            label: label.to_string(),
            date,
            altar_color: "white".to_string(),
            priority: 2,
        });
    }

    // 3. Epiphany series (first event white, the rest green), priority = 3.
    // If the candidate New Year date was omitted, start Epiphany on that candidate date;
    // otherwise, use the first Sunday on or after January 6.
    let epiphany_start = if new_year_candidate < new_year_threshold {
        new_year_candidate
    } else {
        first_sunday_on_or_after(NaiveDate::from_ymd(lit_year + 1, 1, 6))
    };
    for i in 0..=6 {
        let label = if i == 0 {
            "epiphany".to_string()
        } else {
            format!("epiphany + {}", i)
        };
        let color = if i == 0 { "white" } else { "green" };
        insert_event(Event {
            label,
            date: epiphany_start + Duration::days(7 * i as i64),
            altar_color: color.to_string(),
            priority: 3,
        });
    }

    // 4. Pre–Easter series (9 events) with given colors, priority = 4.
    // Labeled "easter - X" (X = 9 down to 1).
    let pre_easter_colors = [
        "green", "green", "white", "purple", "purple", "purple", "purple", "white", "white",
    ];
    let easter = compute_easter(lit_year + 1);
    for j in 1..=9 {
        let offset = 7 * j;
        let date = easter - Duration::days(offset as i64);
        let color = pre_easter_colors[(9 - j) as usize];
        let label = format!("easter - {}", j);
        insert_event(Event {
            label,
            date,
            altar_color: color.to_string(),
            priority: 4,
        });
    }

    // 5. Easter series (7 events, all white), priority = 5.
    for i in 0..=6 {
        let label = if i == 0 {
            "easter".to_string()
        } else {
            format!("easter + {}", i)
        };
        let date = easter + Duration::days(7 * i as i64);
        insert_event(Event {
            label,
            date,
            altar_color: "white".to_string(),
            priority: 5,
        });
    }

    // 6. Pentecost (red), priority = 6.
    let pentecost = easter + Duration::days(49); // 7 weeks after Easter
    insert_event(Event {
        label: "pentecost".to_string(),
        date: pentecost,
        altar_color: "red".to_string(),
        priority: 6,
    });

    // 7. Trinity series (28 events), priority = 7.
    let trinity_start = pentecost + Duration::days(7);
    for i in 0..=27 {
        let label = if i == 0 {
            "trinity".to_string()
        } else {
            format!("trinity + {}", i)
        };
        let date = trinity_start + Duration::days(7 * i as i64);
        let color = if i == 0 {
            "white"
        } else if (1..=4).contains(&i) {
            "green"
        } else if i == 5 {
            "red"
        } else {
            "green"
        };
        insert_event(Event {
            label,
            date,
            altar_color: color.to_string(),
            priority: 7,
        });
    }

    let mut events: Vec<Event> = events_map.into_values().collect();
    events.sort_by_key(|ev| ev.date);
    events
}

/// Determines the liturgical year for an input date.
/// If the input date is on or after the First Sunday of Advent for that calendar year,
/// the liturgical year is the calendar year; otherwise it is the previous calendar year.
fn compute_liturgical_year(input: NaiveDate) -> i32 {
    let candidate = first_sunday_of_advent(input.year());
    if input >= candidate {
        input.year()
    } else {
        input.year() - 1
    }
}

/// Computes the set number from the liturgical year.
/// According to our rule:
///   Advent 2024 → set I, 2025 → set II, 2026 → set III, then repeat.
fn compute_set(lit_year: i32) -> i32 {
    (((lit_year - 2024).rem_euclid(3)) + 1)
}

fn main() {
    let args = Args::parse();

    // Parse the input date.
    let input_date = match NaiveDate::parse_from_str(&args.date, "%d/%m/%Y") {
        Ok(d) => d,
        Err(_) => {
            eprintln!("Error: Unable to parse date. Please use dd/mm/yyyy format.");
            std::process::exit(1);
        }
    };

    // Define a mapping for custom Bible readings.
    // Key: (event label, set number)
    // Value: (Old Testament, Lection, Gospel, Preaching)
    let custom_readings: HashMap<(String, i32), (String, String, String, String)> =
        HashMap::from([
            (
                ("epiphany + 5".to_string(), 1),
                (
                    "Jer 17:5-10".to_string(),
                    "Col 3:12-17".to_string(),
                    "Mat 13:31-35".to_string(),
                    "Mat 13:24-30".to_string(),
                ),
            ),
            // Add more custom entries here as needed.
            (
                ("easter - 9".to_string(), 1),
                (
                    "Jer 1:4-10".to_string(),
                    "1 Cor:09:24-10:05".to_string(),
                    "Mat 19:27-30".to_string(),
                    "Mat 20:1-16".to_string(),
                ),
            ),
        ]);

    // Determine the liturgical year and set.
    let lit_year = compute_liturgical_year(input_date);
    let set = compute_set(lit_year);

    // Generate events for the liturgical year.
    let events = generate_events(lit_year);

    // Look for an event exactly matching the input date.
    if let Some(ev) = events.iter().find(|ev| ev.date == input_date) {
        println!("Date: {}", input_date.format("%d/%m/%Y"));
        println!("Liturgical Year: {}", lit_year);
        println!("Set: {}", set);
        println!("Pericope: {}", ev.label);
        println!("Altar Color: {}", ev.altar_color);
        println!("Readings:");
        
        // Check if a custom Bible reading exists for (event, set).
        let key = (ev.label.clone(), set);
        if let Some((ot, le, go, pr)) = custom_readings.get(&key) {
            println!("  Old Testament: {}", ot);
            println!("  Lection:       {}", le);
            println!("  Gospel:        {}", go);
            println!("  Preaching:     {}", pr);
        } else {
            // Otherwise, print default placeholder readings.
            let gospel_set = if set == 3 { 1 } else { set + 1 };
            println!(
                "  Old Testament: Old Testament reading for {} (Set {})",
                ev.label, set
            );
            println!(
                "  Lection:       Lection reading for {} (Set {})",
                ev.label, set
            );
            println!(
                "  Gospel:        Gospel reading for {} (Set {})",
                ev.label, gospel_set
            );
            println!(
                "  Preaching:     Preaching reading for {} (Set {})",
                ev.label, set
            );
        }
    } else {
        // If no exact match is found, use the most recent Sunday event.
        if let Some(ev) = events.iter().rev().find(|ev| ev.date <= input_date) {
            println!(
                "Note: {} is not an exact event date. Using readings for {} ({}).",
                input_date.format("%d/%m/%Y"),
                ev.label,
                ev.date.format("%d/%m/%Y")
            );
            println!("Liturgical Year: {}", lit_year);
            println!("Set: {}", set);
            println!("Pericope: {}", ev.label);
            println!("Altar Color: {}", ev.altar_color);
            println!("Readings:");
            let gospel_set = if set == 3 { 1 } else { set + 1 };
            println!(
                "  Old Testament: Old Testament reading for {} (Set {})",
                ev.label, set
            );
            println!(
                "  Lection:       Lection reading for {} (Set {})",
                ev.label, set
            );
            println!(
                "  Gospel:        Gospel reading for {} (Set {})",
                ev.label, gospel_set
            );
            println!(
                "  Preaching:     Preaching reading for {} (Set {})",
                ev.label, set
            );
        } else {
            println!(
                "No pericope event found for {} in the liturgical year {}.",
                input_date.format("%d/%m/%Y"),
                lit_year
            );
        }
    }
}
