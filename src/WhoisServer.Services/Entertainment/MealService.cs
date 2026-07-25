using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace WhoisServer.Services.Entertainment;

/// <summary>
/// Ported from src/services/meal.rs — the "what to eat today" suggestion service. Has two
/// independent variants, mirroring the Rust <c>QueryType::Meal</c> / <c>QueryType::MealCN</c>
/// (both parameterless — there is no per-query parameter, the trigger phrase/suffix itself is the
/// whole query):
///  - <see cref="ProcessAsync"/>: a random recipe from TheMealDB's public API (international).
///  - <see cref="ProcessCnAsync"/>: a random recipe from a bundled Chinese recipe dataset
///    (<c>recipes.json</c>, sourced from the "程序员做饭指南"/HowToCook GitHub project). The Rust
///    version embeds this file at compile time via <c>include_str!("../../data/recipes.json")</c>;
///    this port embeds the same file as a .NET manifest resource (see the doc comment on
///    <see cref="LoadChineseRecipes"/> for the required csproj wiring, which is NOT included in
///    this file since editing .csproj files was out of scope for this task).
/// </summary>
public sealed class MealService(HttpClient httpClient)
{
    private const string RandomMealUrl = "https://www.themealdb.com/api/json/v1/1/random.php";

    /// <summary>Regular "今天吃什么" / -MEAL query: one random meal from TheMealDB.</summary>
    public async Task<string> ProcessAsync(CancellationToken ct = default)
    {
        try
        {
            using var response = await httpClient.GetAsync(RandomMealUrl, ct);
            response.EnsureSuccessStatusCode();

            var data = await response.Content.ReadFromJsonAsync<MealResponse>(cancellationToken: ct);
            var meal = data?.Meals?.FirstOrDefault();

            if (meal is null)
            {
                return "% No meal suggestion found. Please try again.\n% Source: TheMealDB (https://www.themealdb.com/)\n";
            }

            return FormatMeal(meal);
        }
        catch (Exception e)
        {
            return $"% Meal Query Failed\n% Error: {e.Message}\n% Source: TheMealDB (https://www.themealdb.com/)\n";
        }
    }

    /// <summary>Chinese "今天吃什么中国" / -MEAL-CN query: one random recipe from the bundled
    /// HowToCook dataset. Synchronous under the hood (no I/O once the resource is cached in
    /// memory) but returns Task for symmetry with <see cref="ProcessAsync"/> and the rest of the
    /// service surface in this codebase.</summary>
    public Task<string> ProcessCnAsync(CancellationToken ct = default)
    {
        try
        {
            var recipes = LoadChineseRecipes();
            if (recipes.Count == 0)
            {
                return Task.FromResult("% No Chinese recipes available in the bundled dataset.\n");
            }

            var (category, name, recipe) = recipes[Random.Shared.Next(recipes.Count)];
            return Task.FromResult(FormatChineseMeal(category, name, recipe));
        }
        catch (Exception e)
        {
            return Task.FromResult($"% Chinese Meal Query Failed\n% Error: {e.Message}\n");
        }
    }

    private static string FormatMeal(Meal meal)
    {
        var sb = new StringBuilder();
        sb.Append("% Meal Suggestion from TheMealDB\n% https://www.themealdb.com/\n\n");
        sb.Append($"meal-id:            {meal.IdMeal}\n");
        sb.Append($"meal-name:          {meal.StrMeal}\n");

        if (meal.StrCategory is { Length: > 0 }) sb.Append($"category:           {meal.StrCategory}\n");
        if (meal.StrArea is { Length: > 0 }) sb.Append($"cuisine:            {meal.StrArea}\n");
        if (meal.StrTags is { Length: > 0 }) sb.Append($"tags:               {meal.StrTags}\n");

        var ingredients = meal.GetIngredients();
        if (ingredients.Count > 0)
        {
            sb.Append("\n% Ingredients\n");
            foreach (var ingredient in ingredients)
            {
                sb.Append($"ingredient:         {ingredient}\n");
            }
        }

        if (meal.StrInstructions is { Length: > 0 } instructions)
        {
            sb.Append("\n% Instructions\n");
            var step = 1;
            foreach (var line in instructions.Replace("\r", "").Split('\n'))
            {
                var trimmed = line.Trim();
                if (trimmed.Length == 0)
                {
                    continue;
                }

                sb.Append($"instruction-{step}:     {trimmed}\n");
                step++;
            }
        }

        if (meal.StrYoutube is { Length: > 0 }) sb.Append($"\nyoutube-video:      {meal.StrYoutube}\n");
        if (meal.StrMealThumb is { Length: > 0 }) sb.Append($"meal-image:         {meal.StrMealThumb}\n");

        sb.Append("\n% Query: 今天吃什么 or -MEAL\n% Powered by TheMealDB API\n");
        return sb.ToString();
    }

    private static string FormatChineseMeal(string category, string name, ChineseRecipe recipe)
    {
        var sb = new StringBuilder();
        sb.Append("% 中国菜谱 - Chinese Recipe\n% 数据来源：程序员做饭指南\n\n");
        sb.Append($"dish-name:          {name}\n");
        sb.Append($"category:           {category}\n");

        if (recipe.Difficulty is { } difficulty)
        {
            sb.Append($"difficulty:         {difficulty} / 10\n");
        }

        if (recipe.Description is { Count: > 0 })
        {
            sb.Append("\n% 描述 (Description)\n");
            foreach (var desc in recipe.Description) sb.Append($"description:        {desc}\n");
        }

        if (recipe.IngredientsTools is { Count: > 0 })
        {
            sb.Append("\n% 原料和工具 (Ingredients & Tools)\n");
            var i = 1;
            foreach (var ingredient in recipe.IngredientsTools) sb.Append($"ingredient-{i++}:      {ingredient}\n");
        }

        if (recipe.IngredientAmounts is { Count: > 0 })
        {
            sb.Append("\n% 食材用量 (Ingredient Amounts)\n");
            var i = 1;
            foreach (var amount in recipe.IngredientAmounts) sb.Append($"amount-{i++}:          {amount}\n");
        }

        if (recipe.Steps is { Count: > 0 })
        {
            sb.Append("\n% 操作步骤 (Cooking Steps)\n");
            var i = 1;
            foreach (var step in recipe.Steps) sb.Append($"step-{i++}:            {step}\n");
        }

        if (recipe.Additional is { Count: > 0 })
        {
            sb.Append("\n% 附加信息 (Additional Info)\n");
            foreach (var info in recipe.Additional) sb.Append($"info:               {info}\n");
        }

        sb.Append("\n% Query: 今天吃什么中国 or -MEAL-CN\n");
        sb.Append("% Source: 程序员做饭指南 https://github.com/Anduin2017/HowToCook\n");
        return sb.ToString();
    }

    private static List<(string Category, string Name, ChineseRecipe Recipe)>? _cachedRecipes;

    /// <summary>Loads and flattens the bundled Chinese recipe dataset, caching the result process-
    /// wide (the dataset is immutable, embedded at build time).
    /// <para><b>Requires a csproj change this task deliberately did not make</b> (out of scope —
    /// see the "hard constraints" this port was given): add to
    /// <c>WhoisServer.Services.csproj</c>:
    /// <code>
    /// &lt;ItemGroup&gt;
    ///   &lt;EmbeddedResource Include="Entertainment/recipes.json"
    ///                       LogicalName="WhoisServer.Services.Entertainment.recipes.json" /&gt;
    /// &lt;/ItemGroup&gt;
    /// </code>
    /// The 621KB source file has already been copied to
    /// <c>src/WhoisServer.Services/Entertainment/recipes.json</c> (verbatim copy of the Rust
    /// project's <c>data/recipes.json</c>) so only the csproj entry is outstanding.</para>
    /// </summary>
    private static List<(string Category, string Name, ChineseRecipe Recipe)> LoadChineseRecipes()
    {
        if (_cachedRecipes is not null)
        {
            return _cachedRecipes;
        }

        using var stream = typeof(MealService).Assembly.GetManifestResourceStream(
            "WhoisServer.Services.Entertainment.recipes.json")
            ?? throw new InvalidOperationException(
                "recipes.json embedded resource not found — see LoadChineseRecipes doc comment for the required csproj wiring");

        var data = JsonSerializer.Deserialize<Dictionary<string, Dictionary<string, ChineseRecipe>>>(stream)
            ?? throw new InvalidOperationException("Failed to parse recipes.json");

        var all = new List<(string, string, ChineseRecipe)>();
        foreach (var (category, dishes) in data)
        {
            foreach (var (name, recipe) in dishes)
            {
                all.Add((category, name, recipe));
            }
        }

        _cachedRecipes = all;
        return all;
    }

    private sealed record MealResponse([property: JsonPropertyName("meals")] List<Meal>? Meals);

    private sealed record Meal(
        [property: JsonPropertyName("idMeal")] string IdMeal,
        [property: JsonPropertyName("strMeal")] string StrMeal,
        [property: JsonPropertyName("strCategory")] string? StrCategory,
        [property: JsonPropertyName("strArea")] string? StrArea,
        [property: JsonPropertyName("strInstructions")] string? StrInstructions,
        [property: JsonPropertyName("strMealThumb")] string? StrMealThumb,
        [property: JsonPropertyName("strTags")] string? StrTags,
        [property: JsonPropertyName("strYoutube")] string? StrYoutube,
        [property: JsonPropertyName("strIngredient1")] string? StrIngredient1,
        [property: JsonPropertyName("strIngredient2")] string? StrIngredient2,
        [property: JsonPropertyName("strIngredient3")] string? StrIngredient3,
        [property: JsonPropertyName("strIngredient4")] string? StrIngredient4,
        [property: JsonPropertyName("strIngredient5")] string? StrIngredient5,
        [property: JsonPropertyName("strIngredient6")] string? StrIngredient6,
        [property: JsonPropertyName("strIngredient7")] string? StrIngredient7,
        [property: JsonPropertyName("strIngredient8")] string? StrIngredient8,
        [property: JsonPropertyName("strIngredient9")] string? StrIngredient9,
        [property: JsonPropertyName("strIngredient10")] string? StrIngredient10,
        [property: JsonPropertyName("strMeasure1")] string? StrMeasure1,
        [property: JsonPropertyName("strMeasure2")] string? StrMeasure2,
        [property: JsonPropertyName("strMeasure3")] string? StrMeasure3,
        [property: JsonPropertyName("strMeasure4")] string? StrMeasure4,
        [property: JsonPropertyName("strMeasure5")] string? StrMeasure5,
        [property: JsonPropertyName("strMeasure6")] string? StrMeasure6,
        [property: JsonPropertyName("strMeasure7")] string? StrMeasure7,
        [property: JsonPropertyName("strMeasure8")] string? StrMeasure8,
        [property: JsonPropertyName("strMeasure9")] string? StrMeasure9,
        [property: JsonPropertyName("strMeasure10")] string? StrMeasure10)
    {
        public List<string> GetIngredients()
        {
            (string? Ingredient, string? Measure)[] pairs =
            [
                (StrIngredient1, StrMeasure1), (StrIngredient2, StrMeasure2), (StrIngredient3, StrMeasure3),
                (StrIngredient4, StrMeasure4), (StrIngredient5, StrMeasure5), (StrIngredient6, StrMeasure6),
                (StrIngredient7, StrMeasure7), (StrIngredient8, StrMeasure8), (StrIngredient9, StrMeasure9),
                (StrIngredient10, StrMeasure10),
            ];

            var result = new List<string>();
            foreach (var (ingredient, measure) in pairs)
            {
                if (string.IsNullOrWhiteSpace(ingredient))
                {
                    continue;
                }

                var line = ingredient.Trim();
                if (!string.IsNullOrWhiteSpace(measure))
                {
                    line = $"{measure.Trim()} - {line}";
                }

                result.Add(line);
            }

            return result;
        }
    }

    private sealed record ChineseRecipe(
        [property: JsonPropertyName("描述")] List<string>? Description,
        [property: JsonPropertyName("预估烹饪难度")] int? Difficulty,
        [property: JsonPropertyName("原料和工具")] List<string>? IngredientsTools,
        [property: JsonPropertyName("食材计算")] List<string>? IngredientAmounts,
        [property: JsonPropertyName("操作步骤")] List<string>? Steps,
        [property: JsonPropertyName("附加内容")] List<string>? Additional);
}
