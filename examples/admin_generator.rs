// ============================================================
// مثال: Admin Generator | Example: Admin Scaffolding
// ============================================================

use noor::*;
use noor::core::admin::{ScaffoldBuilder, FieldBuilder, FieldType};

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🏗️ Admin Generator Demo\n");
    
    // Build scaffold configuration
    let generator = ScaffoldBuilder::new("Product")
        .table("products")
        .field(
            FieldBuilder::new("name", FieldType::String)
                .label("Product Name")
                .required()
                .searchable()
                .sortable()
                .validation("min:2", "Name must be at least 2 characters")
                .validation("max:100", "Name must not exceed 100 characters")
                .build()
        )
        .field(
            FieldBuilder::new("description", FieldType::Textarea)
                .label("Description")
                .in_list(false)
                .build()
        )
        .field(
            FieldBuilder::new("price", FieldType::Float)
                .label("Price")
                .required()
                .sortable()
                .validation("min:0", "Price must be positive")
                .build()
        )
        .field(
            FieldBuilder::new("stock", FieldType::Integer)
                .label("Stock Quantity")
                .required()
                .sortable()
                .build()
        )
        .field(
            FieldBuilder::new("status", FieldType::Select)
                .label("Status")
                .required()
                .options(vec![
                    ("draft".to_string(), "Draft".to_string()),
                    ("active".to_string(), "Active".to_string()),
                    ("inactive".to_string(), "Inactive".to_string()),
                ])
                .build()
        )
        .field(
            FieldBuilder::new("featured", FieldType::Boolean)
                .label("Featured Product")
                .in_list(false)
                .build()
        )
        .field(
            FieldBuilder::new("image", FieldType::Image)
                .label("Product Image")
                .in_list(false)
                .build()
        )
        .per_page(20)
        .searchable(true)
        .sortable(true)
        .build();
    
    // Generate all files
    let files = generator.generate_all();
    
    println!("✓ Generated Controller ({} bytes)", files.controller.len());
    println!("✓ Generated List View ({} bytes)", files.list_view.len());
    println!("✓ Generated Form View ({} bytes)", files.form_view.len());
    println!("✓ Generated Show View ({} bytes)", files.show_view.len());
    
    // Print a preview of the controller
    println!("\n📋 Controller Preview (first 500 chars):");
    let preview: String = files.controller.chars().take(500).collect();
    println!("{}", preview);
    println!("...");
    
    // Save files to disk
    std::fs::create_dir_all("generated/admin/products")?;
    std::fs::write("generated/admin/products/controller.rs", &files.controller)?;
    std::fs::write("generated/admin/products/list.hbs", &files.list_view)?;
    std::fs::write("generated/admin/products/form.hbs", &files.form_view)?;
    std::fs::write("generated/admin/products/show.hbs", &files.show_view)?;
    
    println!("\n💾 Files saved to: generated/admin/products/");
    println!("\n✅ Admin Generator demo completed!");
    Ok(())
}
