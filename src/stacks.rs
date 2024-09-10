use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use termion::color::{self, Color};

const COLORS: [&str; 10] = [
    "red",
    "yellow",
    "blue",
    "green",
    "cyan",
    "light_red",
    "light_yellow",
    "light_blue",
    "light_green",
    "light_cyan",
];

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Stack {
    pub name: String,
    pub color: String,
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let stack_color: &dyn Color = match self.color.to_lowercase().as_str() {
            "red" => &color::Red,
            "yellow" => &color::Yellow,
            "blue" => &color::Blue,
            "green" => &color::Green,
            "cyan" => &color::Cyan,
            "light_red" => &color::LightRed,
            "light_yellow" => &color::LightYellow,
            "light_blue" => &color::LightBlue,
            "light_green" => &color::LightGreen,
            "ligh_cyan" => &color::LightCyan,
            _ => &color::Red, // default to Red if color name is unrecognized
        };
        write!(
            f,
            "{}[{}]{}",
            color::Fg(stack_color),
            self.name,
            color::Fg(color::Reset)
        )
    }
}

impl Stack {
    pub fn new(new_stack_name: &str, stacks: &Vec<Stack>) -> Result<Self> {
        // Check if new stack name is reserved
        if new_stack_name == "all" {
            bail!("Name reserved for base stack")
        }
        //check if its already taken
        stacks.iter().for_each(|s| {
            if s.name == new_stack_name {
                anyhow!("Stack {} already exists", new_stack_name);
            };
        });
        // Count the usage of each color
        let mut color_usage = HashMap::new();
        for color in &COLORS {
            color_usage.insert(*color, 0);
        }
        stacks
            .iter()
            .for_each(|s| *color_usage.entry(s.color.as_str()).or_insert(0) += 1);
        // Find the least used color, respecting the order in COLORS
        let chosen_color = COLORS
            .iter()
            .min_by_key(|&color| (color_usage[color], COLORS.iter().position(|&c| c == *color)))
            .unwrap();
        // Create and return the new Stack
        Ok(Stack {
            name: new_stack_name.to_string(),
            color: chosen_color.to_string(),
        })
    }
}
