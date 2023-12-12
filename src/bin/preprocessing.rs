use polars::prelude::*;
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = CsvReader::from_path("amazon.csv")?
        .infer_schema(None)
        .finish()?;

    let mut new_df = df
        .select(&[
            "product_id",
            "user_id",
            "category",
            "actual_price",
            "product_name",
        ])
        .unwrap();

    // Split "category" by "|" and take the first part
    let category_col = new_df
        .column("category")?
        .utf8()?
        .into_iter()
        .map(|opt_val| opt_val.map(|val| val.split('|').next().unwrap_or_default()))
        .collect::<Utf8Chunked>();

    let new_df = new_df.with_column(category_col).unwrap();

    // Remove "₹" and "," from "actual_price", convert to float and normalize
    let actual_price_col = new_df
        .column("actual_price")?
        .utf8()?
        .into_iter()
        .map(|opt_val| {
            opt_val.map(|val| {
                val.replace("₹", "")
                    .replace(",", "")
                    .parse::<f64>()
                    .unwrap_or_default()
            })
        })
        .collect::<Float64Chunked>();
    let max_price = actual_price_col.max().unwrap();
    let mut normalized_price =
        actual_price_col.apply(|price| Some(100.0 * (price.unwrap() / max_price)));
    normalized_price.rename("price");
    let mut new_df = new_df.with_column(normalized_price).unwrap();

    // Explode "user_id" split by ","
    let user_id_col = new_df
        .column("user_id")?
        .utf8()?
        .into_iter()
        .map(|opt_val| opt_val.map(|val| val.split(',').map(|s| s.to_string()).collect::<Series>()))
        .collect::<ListChunked>()
        .into_series();
    user_id_col.explode()?.into_series().rename("user_id");
    new_df = new_df.with_column(user_id_col)?;

    // select last 4 columns
    let mut new_df = new_df
        .select(&["product_id", "collected", "", "price", "product_name"])
        .unwrap();

    new_df.rename("collected", "user_id").unwrap();
    new_df.rename("", "category").unwrap();

    let mut new_df = new_df.explode(["user_id"])?;

    // print the first 5 rows
    println!("{:?}", new_df.head(Some(5)));

    // Write to CSV
    let mut file = File::create("amazon_cleaned.csv")?;
    CsvWriter::new(&mut file).finish(&mut new_df)?;

    Ok(())
}
