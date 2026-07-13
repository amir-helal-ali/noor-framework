// ============================================================
// مثال: i18n | Example: Internationalization
// ============================================================

use noor::*;
use noor::core::i18n::Translator;

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🌍 Internationalization Demo\n");
    
    let translator = Translator::new("ar", "en");
    
    // Load translation files
    translator.load_directory("lang")?;
    
    // Simple translations
    println!("English:");
    println!("  welcome: {}", translator.translate("welcome", Some("en")));
    println!("  login: {}", translator.translate("login", Some("en")));
    println!("  logout: {}", translator.translate("logout", Some("en")));
    
    println!("\nArabic:");
    println!("  welcome: {}", translator.translate("welcome", Some("ar")));
    println!("  login: {}", translator.translate("login", Some("ar")));
    println!("  logout: {}", translator.translate("logout", Some("ar")));
    
    // Translation with parameters
    let mut params = std::collections::HashMap::new();
    params.insert("field".to_string(), "email".to_string());
    
    println!("\nWith parameters:");
    println!("  EN: {}", translator.translate_with_params("validation.required", &params, Some("en")));
    println!("  AR: {}", translator.translate_with_params("validation.required", &params, Some("ar")));
    
    // Pluralization
    println!("\nPluralization:");
    for count in [0, 1, 2, 5, 100] {
        println!("  {} item(s) - EN: '{}' | AR: '{}'", 
            count,
            translator.plural("items", count, Some("en")),
            translator.plural("items", count, Some("ar"))
        );
    }
    
    // Locale detection
    println!("\nLocale detection:");
    let headers = vec![
        "ar,en;q=0.8",
        "en-US,en;q=0.9",
        "fr,de;q=0.5",
        "ar-SA,en;q=0.7",
    ];
    
    for header in headers {
        let detected = translator.detect_locale(header);
        let direction = translator.direction(&detected);
        println!("  '{}' -> {} ({})", header, detected, direction.as_str());
    }
    
    println!("\n✅ i18n demo completed!");
    Ok(())
}
