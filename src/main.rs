use dotenv;
use more_asserts::assert_gt;
use regex::Regex;
use reqwest::blocking;
use scraper::{Html, Selector};
use std::{error::Error, fmt};

#[derive(Debug, Default, Clone)]
struct Food {
    description: String,
    quantity: u32,
    calories: u32,
    proteins: u32,
}

impl Food {
    fn scale_food(&self, scale: f32) -> Food {
        return Food {
            quantity: self.quantity,
            description: self.description.clone(),
            calories: (scale * self.calories as f32).round() as u32,
            proteins: (scale * self.proteins as f32).round() as u32,
        };
    }
}

impl fmt::Display for Food {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = format!("{}\n", self.description);
        s.push_str(format!("Calories: {} kcals\n", self.calories).as_str());
        s.push_str(format!("Proteins: {}g\n", self.proteins).as_str());
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
struct Menu {
    date: String,
    foods: Vec<Food>,
}

impl Menu {
    fn add_food(&mut self, food: Food) {
        self.foods.push(food);
    }

    fn total_calories(&self) -> u32 {
        return self
            .foods
            .iter()
            .map(|food| food.calories)
            .reduce(|acc, calories| acc + calories)
            .unwrap_or_default();
    }

    fn total_proteins(&self) -> u32 {
        return self
            .foods
            .iter()
            .map(|food| food.proteins)
            .reduce(|acc, calories| acc + calories)
            .unwrap_or_default();
    }

    fn add_additional_food(
        &mut self,
        daily_calories: u32,
        daily_proteins: u32,
        additional_food: Vec<Food>,
        food_weights: Vec<f32>,
    ) {
        let current_calories = self.total_calories();
        if current_calories >= daily_calories {
            println!(
                "The menu has a total of {}(+{}) = {} kcals. No additional food is needed.",
                daily_calories,
                current_calories - daily_calories,
                current_calories
            );
        } else {
            println!(
                "The menu has a total of {}(-{}) = {} kcals. Computing additional food ...",
                daily_calories,
                daily_calories - current_calories,
                current_calories
            );
            for (food, weight) in Iterator::zip(additional_food.iter(), food_weights) {
                let calories_to_add = weight * (daily_calories - current_calories) as f32;
                let scale = calories_to_add / food.calories as f32;
                let new_food = food.scale_food(scale);
                println!(
                    "Added food \"{}\" with weight {} grams",
                    new_food.description, new_food.quantity
                );
                self.add_food(new_food);
            }
            let new_calories = self.total_calories();
            println!(
                "The new menu has a total of {}(+{}) = {} kcals. No additional food is needed.",
                daily_calories,
                new_calories - daily_calories,
                new_calories
            );
        }
        let current_proteins = self.total_proteins();
        if current_proteins >= daily_proteins {
            println!(
                "The menu has a total of {}(+{}) = {}g of proteins.",
                daily_proteins,
                current_proteins - daily_proteins,
                current_proteins
            );
        } else {
            println!(
                "The menu has a total of {}(-{}) = {}g of proteins.",
                daily_proteins,
                daily_proteins - current_proteins,
                current_proteins
            );
        }
    }
}

impl fmt::Display for Menu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = format!("Menu for date {}\n", self.date);
        s.push_str(format!("{:-<60}\n", "").as_str());
        for food in &self.foods {
            s.push_str(format!("{}", food).as_str());
            s.push_str(format!("{:-<60}\n", "").as_str());
        }
        s.push_str(format!("Total menu calories {} kcals\n", self.total_calories()).as_str());
        s.push_str(format!("Total menu proteins {}g\n", self.total_proteins()).as_str());
        write!(f, "{}", s)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder().build()?;
    let (id, date, program_id) = request_today_arguments(
        &client,
        "https://fitfoodway.ro/programe/creste-masa-musculara",
    )?;

    let mut menu = request_today_menu(
        &client,
        "https://fitfoodway.ro/fitfoodway/detalii_meniu",
        &id,
        &date,
        &program_id,
    )?;
    let daily_calories = dotenv::var("DAILY_CALORIES")
        .expect("Daily calories not in \".env\" file")
        .parse::<u32>()
        .expect("Daily calories is not in u32 format");
    let daily_proteins = dotenv::var("DAILY_PROTEINS")
        .expect("Daily proteins not in \".env\" file")
        .parse::<u32>()
        .expect("Daily proteins is not in u32 format");
    println!("Total calories for today {} kcals", daily_calories);
    println!("Total proteins for today {}g", daily_proteins);
    println!("{}", menu);
    let additional_food: Vec<Food> = vec![
        Food {
            description: "Chicken breast".to_string(),
            calories: 110,
            proteins: 20,
            quantity: 100,
        },
        Food {
            description: "Whey protein".to_string(),
            calories: 388,
            proteins: 80,
            quantity: 100,
        },
    ];
    // let food_weights = vec![0.7, 0.3];
    let mut food_weights: Vec<f32> = Vec::new();
    for _ in &additional_food {
        food_weights.push(1.0 / additional_food.len() as f32);
    }
    menu.add_additional_food(
        daily_calories,
        daily_proteins,
        additional_food,
        food_weights,
    );
    Ok(())
}

fn request_today_menu(
    client: &blocking::Client,
    url: &str,
    id: &str,
    date: &str,
    program_id: &str,
) -> Result<Menu, reqwest::Error> {
    let body = client
        .post(url)
        .form(&[("id", id), ("data", date), ("program_id", program_id)])
        .send()?
        .text()?;
    let parsed_html = Html::parse_document(&body);
    let selector = &Selector::parse("div.modal-body")
        .expect("Selector for \"Detalii > modal-body\" is not valid.");
    let menu_text: Vec<&str> = parsed_html
        .select(selector)
        .next()
        .expect("Button \"Detalii\" is not on page.")
        .text()
        .skip(1)
        .collect();
    let mut menu = Menu {
        date: date.to_string(),
        foods: Vec::<Food>::new(),
    };
    let quantity_regex =
        Regex::new(r"Gramaje?\s*:?\s*([0-9]+)\s*[gm]").expect("Regex for quantity is invalid");
    let kcal_regex = Regex::new(r"([0-9]+)\s*kcal").expect("Regex for kcal is invalid");
    let proteins_regex =
        Regex::new(r"proteine\s*:?\s*([0-9]+)\s*g").expect("Regex for proteins is invalid");
    let description_regex =
        Regex::new(r"^\n[^*][^:0-9]+:[^:0-9]+$").expect("Regex for description is invalid");
    let mut food = Food::default();
    let mut mask = 0;
    for slice in menu_text {
        if let Some(description) = extract_description(&description_regex, slice) {
            println!("found description at {}", slice);
            if (mask & (1 << 0)) > 0 {
                panic!("description two times");
            }
            mask |= 1 << 0;
            food.description = description;
        }
        if let Some(quantity) = extract_u32(&quantity_regex, slice) {
            println!("found quant at {}", slice);
            if (mask & (1 << 1)) > 0 {
                panic!("quantity two times");
            }
            mask |= 1 << 1;
            food.quantity = quantity;
        }
        if let Some(calories) = extract_u32(&kcal_regex, slice) {
            println!("found cal at {}", slice);
            if (mask & (1 << 2)) > 0 {
                panic!("calories two times");
            }
            mask |= 1 << 2;
            food.calories = calories;
        }
        if let Some(proteins) = extract_u32(&proteins_regex, slice) {
            println!("found oriot at {}", slice);
            if (mask & (1 << 3)) > 0 {
                panic!("proteins two times");
            }
            mask |= 1 << 3;
            food.proteins = proteins;
        }
        if mask == (1 << 4) - 1 {
            assert_ne!(food.description, "");
            assert_gt!(food.quantity, 0);
            assert_gt!(food.calories, 0);
            assert_gt!(food.proteins, 0);
            menu.add_food(food.clone());
            mask = 0;
        }
    }
    if mask != 0 {
        panic!("Final mask is not empty. Remaining attributes unused.")
    }
    Ok(menu)
}

fn extract_description(regex: &Regex, s: &str) -> Option<String> {
    if !s.starts_with("\n-") {
        if regex.is_match(s) {
            return Some(s[1..].to_string());
        }
        return None;
    }
    return Some(s[2..].to_string());
}

fn extract_u32(regex: &Regex, s: &str) -> Option<u32> {
    let caps = regex.captures(s)?;
    Some(
        caps.get(1)?
            .as_str()
            .parse::<u32>()
            .expect("Extracted match is not a number"),
    )
}

fn parse_today_arguments(args: &str) -> (String, String, String) {
    let re = Regex::new(r"\(([0-9]+), '([0-9\-]+)', '([0-9]+)'\)")
        .expect("Regex for today arguments is invalid");
    let caps: Vec<&str> = re
        .captures(args)
        .expect("The \"Detalii\" arguments does not match the regex")
        .iter()
        .skip(1)
        .map(|capture| capture.unwrap().as_str())
        .collect();

    if caps.len() != 3 {
        panic!("Should have captured 3 arguments but got {}", caps.len());
    }
    let id = caps[0].to_owned();
    let date = caps[1].to_owned();
    let program_id = caps[2].to_owned();
    (id, date, program_id)
}

fn request_today_arguments(
    client: &blocking::Client,
    url: &str,
) -> Result<(String, String, String), reqwest::Error> {
    let body = client.get(url).send()?.text()?;
    let parsed_html = Html::parse_document(&body);
    let selector =
        &Selector::parse("div.btn-detalii > a").expect("Selector for \"Detalii\" is not valid.");
    let btn_element = parsed_html
        .select(selector)
        .next()
        .expect("Button \"Detalii\" is not on page.");
    let onclick_text = btn_element
        .value()
        .attr("onclick")
        .expect("Button \"Detalii\" has onclick event");
    Ok(parse_today_arguments(onclick_text))
}
