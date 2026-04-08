use ripweb::error::RipwebError;
use ripweb::extract::{web::WebExtractor, Extractor};

fn main() {
    println!("\n=== TORTURE TEST RESULTS ===\n");

    // 1. deeply_nested_divs.html
    println!("1. deeply_nested_divs.html (110KB, 10K nested divs)");
    let bytes1 = std::fs::read("corpus/torture/deeply_nested_divs.html").unwrap();
    let result1 = WebExtractor::extract(&bytes1, Some("text/html"));
    match result1 {
        Ok(text) => {
            println!("   ✓ PASSED - Extracted {} chars", text.len());
            if text.contains("innermost") || text.contains("real content") {
                println!("   ✓ Found innermost content");
            }
        }
        Err(e) => println!("   ✗ FAILED - {:?}", e),
    }
    println!();

    // 2. million_links.html
    println!("2. million_links.html (328KB, 10K links)");
    let bytes2 = std::fs::read("corpus/torture/million_links.html").unwrap();
    let result2 = WebExtractor::extract(&bytes2, Some("text/html"));
    match result2 {
        Ok(text) => {
            println!("   ✓ PASSED - Extracted {} chars", text.len());
            // nav links should be nuked
            if !text.contains("Link 0") && !text.contains("Link 9999") {
                println!("   ✓ Navigation links properly nuked");
            }
            // real content should remain
            if text.contains("real content") {
                println!("   ✓ Main content preserved");
            }
        }
        Err(e) => println!("   ✗ FAILED - {:?}", e),
    }
    println!();

    // 3. giant_inline_svg.html (6.2MB - over 5MB limit)
    println!("3. giant_inline_svg.html (6.2MB - TESTING 5MB LIMIT)");
    let bytes3 = std::fs::read("corpus/torture/giant_inline_svg.html").unwrap();
    println!("   Input size: {} bytes", bytes3.len());
    let result3 = WebExtractor::extract(&bytes3, Some("text/html"));
    match result3 {
        Ok(_) => println!("   ✗ FAILED - Should have rejected oversized input!"),
        Err(RipwebError::InputTooLarge(size)) => {
            println!("   ✓ PASSED - InputTooLarge correctly detected!");
            println!("   ✓ Rejected {} bytes (over 5MB limit)", size);
        }
        Err(e) => println!("   ✗ FAILED - Wrong error type: {:?}", e),
    }
    println!();

    // 4. binary_disguised_as_html.html
    println!("4. binary_disguised_as_html.html (10KB with invalid UTF-8)");
    let bytes4 =
        std::fs::read("corpus/torture/binary_disguised_as_html.html").unwrap();
    let result4 = WebExtractor::extract(&bytes4, Some("text/html"));
    match result4 {
        Ok(text) => {
            println!("   ✓ PASSED - Gracefully handled binary data");
            println!(
                "   ✓ Extracted {} chars (with replacement chars)",
                text.len()
            );
        }
        Err(e) => println!("   ✗ FAILED - {:?}", e),
    }

    println!("\n=== ALL TESTS COMPLETE ===");
}
