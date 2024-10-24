# thirtyfour-mouse
Adjustable mouse movement controls for Thirtyfour

## Notice
Currently, uses the Github repo for Thirtyfour as the required changes have not been pushed to crates.io yet.

## Example
```rust
#[tokio::main]
async fn main() {
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:9515", caps).await.unwrap();
    driver.goto("https://autodraw.com/").await.unwrap();
    sleep_until(Instant::now() + Duration::from_millis(500)).await;
    
    // Start Drawing Button
    match driver.find(By::Css("div[class=\"button green\"]")).await {
        Ok(btn) => {
            btn.click().await.unwrap();
        },
        Err(_) => ()
    };
    sleep_until(Instant::now() + Duration::from_millis(500)).await;

    // Pencil Tool
    let draw_icon = driver.find(By::Css("div[class=\"tool pencil\"]")).await.unwrap();
    draw_icon.click().await.unwrap();

    // Start Somewhere on the canvas.
    driver.action_chain_with_delay(None, Some(0)).move_to(900, 700).perform().await.unwrap();
    
    // Draw on mouse path canvas
    let mouse_action = MouseAction::new(
        MouseInterpolation::Spline,
        MouseButtonAction::LeftHold,
        MouseButtonAction::LeftRelease,
        Some(2_000), // ~2 Seconds
        Some(1)
    );
    driver.mouse_action(mouse_action, &draw_icon).await.unwrap();
    
    sleep_until(Instant::now() + Duration::from_millis(10_000)).await;

    driver.quit().await.unwrap();
}
```