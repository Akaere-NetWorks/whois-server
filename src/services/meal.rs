/*
 * Copyright (C) 2024 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use anyhow::Result;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, Deserialize, Serialize)]
struct MealResponse {
    meals: Option<Vec<Meal>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Meal {
    #[serde(rename = "idMeal")]
    id_meal: String,
    #[serde(rename = "strMeal")]
    str_meal: String,
    #[serde(rename = "strCategory")]
    str_category: Option<String>,
    #[serde(rename = "strArea")]
    str_area: Option<String>,
    #[serde(rename = "strInstructions")]
    str_instructions: Option<String>,
    #[serde(rename = "strMealThumb")]
    str_meal_thumb: Option<String>,
    #[serde(rename = "strTags")]
    str_tags: Option<String>,
    #[serde(rename = "strYoutube")]
    str_youtube: Option<String>,
    #[serde(rename = "strIngredient1")]
    str_ingredient1: Option<String>,
    #[serde(rename = "strIngredient2")]
    str_ingredient2: Option<String>,
    #[serde(rename = "strIngredient3")]
    str_ingredient3: Option<String>,
    #[serde(rename = "strIngredient4")]
    str_ingredient4: Option<String>,
    #[serde(rename = "strIngredient5")]
    str_ingredient5: Option<String>,
    #[serde(rename = "strIngredient6")]
    str_ingredient6: Option<String>,
    #[serde(rename = "strIngredient7")]
    str_ingredient7: Option<String>,
    #[serde(rename = "strIngredient8")]
    str_ingredient8: Option<String>,
    #[serde(rename = "strIngredient9")]
    str_ingredient9: Option<String>,
    #[serde(rename = "strIngredient10")]
    str_ingredient10: Option<String>,
    #[serde(rename = "strMeasure1")]
    str_measure1: Option<String>,
    #[serde(rename = "strMeasure2")]
    str_measure2: Option<String>,
    #[serde(rename = "strMeasure3")]
    str_measure3: Option<String>,
    #[serde(rename = "strMeasure4")]
    str_measure4: Option<String>,
    #[serde(rename = "strMeasure5")]
    str_measure5: Option<String>,
    #[serde(rename = "strMeasure6")]
    str_measure6: Option<String>,
    #[serde(rename = "strMeasure7")]
    str_measure7: Option<String>,
    #[serde(rename = "strMeasure8")]
    str_measure8: Option<String>,
    #[serde(rename = "strMeasure9")]
    str_measure9: Option<String>,
    #[serde(rename = "strMeasure10")]
    str_measure10: Option<String>,
}

impl Meal {
    fn get_ingredients(&self) -> Vec<String> {
        let mut ingredients = Vec::new();
        let ingredient_fields = [
            (&self.str_ingredient1, &self.str_measure1),
            (&self.str_ingredient2, &self.str_measure2),
            (&self.str_ingredient3, &self.str_measure3),
            (&self.str_ingredient4, &self.str_measure4),
            (&self.str_ingredient5, &self.str_measure5),
            (&self.str_ingredient6, &self.str_measure6),
            (&self.str_ingredient7, &self.str_measure7),
            (&self.str_ingredient8, &self.str_measure8),
            (&self.str_ingredient9, &self.str_measure9),
            (&self.str_ingredient10, &self.str_measure10),
        ];

        for (ingredient, measure) in ingredient_fields {
            if let Some(ing) = ingredient
                && !ing.trim().is_empty()
            {
                let mut ingredient_line = ing.trim().to_string();
                if let Some(measure) = measure
                    && !measure.trim().is_empty()
                {
                    ingredient_line = format!("{} - {}", measure.trim(), ingredient_line);
                }
                ingredients.push(ingredient_line);
            }
        }
        ingredients
    }
}

pub async fn query_random_meal() -> Result<String> {
    let client = reqwest::Client::new();
    let url = "https://www.themealdb.com/api/json/v1/1/random.php";

    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "MealDB API returned status: {}",
            response.status()
        ));
    }

    let meal_response: MealResponse = response.json().await?;

    if let Some(meals) = meal_response.meals
        && let Some(meal) = meals.first()
    {
        return Ok(format_meal_info(meal));
    }

    Err(anyhow::anyhow!("No meal found in API response"))
}

fn format_meal_info(meal: &Meal) -> String {
    let mut result = String::new();

    let _ = writeln!(result, "% Meal Information from TheMealDB");
    let _ = writeln!(result, "% https://www.themealdb.com/");
    let _ = writeln!(result);

    let _ = writeln!(result, "meal-id:           {}", meal.id_meal);
    let _ = writeln!(result, "meal-name:         {}", meal.str_meal);

    if let Some(category) = &meal.str_category {
        let _ = writeln!(result, "category:          {}", category);
    }

    if let Some(area) = &meal.str_area {
        let _ = writeln!(result, "cuisine:           {}", area);
    }

    if let Some(tags) = &meal.str_tags
        && !tags.trim().is_empty()
    {
        let _ = writeln!(result, "tags:              {}", tags);
    }

    let ingredients = meal.get_ingredients();
    if !ingredients.is_empty() {
        let _ = writeln!(result);
        let _ = writeln!(result, "% Ingredients");
        for ingredient in ingredients {
            let _ = writeln!(result, "ingredient:        {}", ingredient);
        }
    }

    if let Some(instructions) = &meal.str_instructions
        && !instructions.trim().is_empty()
    {
        let _ = writeln!(result);
        let _ = writeln!(result, "% Instructions");
        let instructions = instructions.replace('\r', "");
        for (i, line) in instructions.lines().enumerate() {
            if !line.trim().is_empty() {
                let _ = writeln!(result, "instruction-{}:     {}", i + 1, line.trim());
            }
        }
    }

    if let Some(youtube) = &meal.str_youtube
        && !youtube.trim().is_empty()
    {
        let _ = writeln!(result);
        let _ = writeln!(result, "youtube-video:     {}", youtube);
    }

    if let Some(image) = &meal.str_meal_thumb
        && !image.trim().is_empty()
    {
        let _ = writeln!(result, "meal-image:        {}", image);
    }

    let _ = writeln!(result);
    let _ = writeln!(result, "% Query: 今天吃什么 or -MEAL");
    let _ = writeln!(result, "% Powered by TheMealDB API");

    result
}

#[derive(Debug, Deserialize)]
struct ChineseRecipe {
    #[serde(rename = "描述")]
    description: Option<Vec<String>>,
    #[serde(rename = "预估烹饪难度")]
    difficulty: Option<u8>,
    #[serde(rename = "原料和工具")]
    ingredients_tools: Option<Vec<String>>,
    #[serde(rename = "食材计算")]
    ingredient_amounts: Option<Vec<String>>,
    #[serde(rename = "操作步骤")]
    steps: Option<Vec<String>>,
    #[serde(rename = "附加内容")]
    additional: Option<Vec<String>>,
}

pub async fn query_random_chinese_meal() -> Result<String> {
    // 读取 recipes.json 文件
    let recipes_content = include_str!("../../data/recipes.json");
    let recipes: HashMap<String, HashMap<String, ChineseRecipe>> =
        serde_json::from_str(recipes_content)?;

    // 收集所有菜谱
    let mut all_recipes = Vec::new();
    for (category, category_recipes) in recipes.iter() {
        for (name, recipe) in category_recipes.iter() {
            all_recipes.push((category.clone(), name.clone(), recipe));
        }
    }

    if all_recipes.is_empty() {
        return Err(anyhow::anyhow!("No recipes found in recipes.json"));
    }

    // 随机选择一个菜谱
    let mut rng = rand::thread_rng();
    let (category, name, recipe) = all_recipes
        .choose(&mut rng)
        .ok_or_else(|| anyhow::anyhow!("Failed to select random recipe"))?;

    Ok(format_chinese_meal_info(category, name, recipe))
}

fn format_chinese_meal_info(category: &str, name: &str, recipe: &ChineseRecipe) -> String {
    let mut result = String::new();

    let _ = writeln!(result, "% 中国菜谱 - Chinese Recipe");
    let _ = writeln!(result, "% 数据来源：程序员做饭指南");
    let _ = writeln!(result);

    let _ = writeln!(result, "dish-name:         {}", name);
    let _ = writeln!(result, "category:          {}", category);

    if let Some(difficulty) = recipe.difficulty {
        let _ = writeln!(result, "difficulty:        {} / 10", difficulty);
    }

    if let Some(descriptions) = &recipe.description
        && !descriptions.is_empty()
    {
        let _ = writeln!(result);
        let _ = writeln!(result, "% 描述 (Description)");
        for desc in descriptions {
            let _ = writeln!(result, "description:       {}", desc);
        }
    }

    if let Some(ingredients) = &recipe.ingredients_tools
        && !ingredients.is_empty()
    {
        let _ = writeln!(result);
        let _ = writeln!(result, "% 原料和工具 (Ingredients & Tools)");
        for (i, ingredient) in ingredients.iter().enumerate() {
            let _ = writeln!(result, "ingredient-{}:      {}", i + 1, ingredient);
        }
    }

    if let Some(amounts) = &recipe.ingredient_amounts
        && !amounts.is_empty()
    {
        let _ = writeln!(result);
        let _ = writeln!(result, "% 食材用量 (Ingredient Amounts)");
        for (i, amount) in amounts.iter().enumerate() {
            let _ = writeln!(result, "amount-{}:          {}", i + 1, amount);
        }
    }

    if let Some(steps) = &recipe.steps
        && !steps.is_empty()
    {
        let _ = writeln!(result);
        let _ = writeln!(result, "% 操作步骤 (Cooking Steps)");
        for (i, step) in steps.iter().enumerate() {
            let _ = writeln!(result, "step-{}:            {}", i + 1, step);
        }
    }

    if let Some(additional) = &recipe.additional
        && !additional.is_empty()
    {
        let _ = writeln!(result);
        let _ = writeln!(result, "% 附加信息 (Additional Info)");
        for info in additional {
            let _ = writeln!(result, "info:              {}", info);
        }
    }

    let _ = writeln!(result);
    let _ = writeln!(result, "% Query: 今天吃什么中国 or -MEAL-CN");
    let _ = writeln!(
        result,
        "% Source: 程序员做饭指南 https://github.com/Anduin2017/HowToCook"
    )
    ;

    result
}
