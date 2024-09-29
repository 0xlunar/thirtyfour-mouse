use async_trait::async_trait;
use enterpolation::bezier::Bezier;
use enterpolation::bspline::BSpline;
use enterpolation::{easing, linear::Linear, Curve};
use rand::{thread_rng, Rng};
use thirtyfour::action_chain::ActionChain;
use thirtyfour::error::{WebDriverError, WebDriverResult};
use thirtyfour::{WebDriver, WebElement};

#[derive(Default, Debug, Clone)]
pub struct MouseAction {
    interpolation: MouseInterpolation,
    start_action: MouseButtonAction,
    end_action: MouseButtonAction,
    duration_ms: u64,
    jitter_amount: i64,
}

#[derive(Default, Debug, Clone)]
pub enum MouseButtonAction {
    #[default]
    None,
    LeftClick,
    LeftHold,
    LeftRelease,
    RightClick,
}

#[derive(Default, Debug, Clone)]
pub enum MouseInterpolation {
    #[default]
    Linear,
    Spline,
}

impl MouseAction {
    pub fn new(
        interpolation: MouseInterpolation,
        start_action: MouseButtonAction,
        end_action: MouseButtonAction,
        duration_ms: Option<u64>,
        jitter_amount: Option<i64>,
    ) -> Self {
        let jitter_amount = jitter_amount.unwrap_or(0);
        let mut duration_ms = duration_ms.unwrap_or(500);

        // Each Action takes between 5-9ms with it averaging out to 7ms
        let divider = 7;
        if duration_ms < divider {
            duration_ms = 1;
        } else {
            duration_ms /= divider;
        }

        MouseAction {
            interpolation,
            start_action,
            end_action,
            duration_ms,
            jitter_amount,
        }
    }
}

#[async_trait]
pub trait MouseActionExt {
    async fn mouse_action(
        &self,
        action: MouseAction,
        target_element: &WebElement,
    ) -> WebDriverResult<()>;
}

#[async_trait]
impl MouseActionExt for WebDriver {
    /// Simulate mouse movement across a path over a duration
    ///
    /// Note: There is no guarantee the duration is exact, but should be close
    async fn mouse_action(
        &self,
        action: MouseAction,
        target_element: &WebElement,
    ) -> WebDriverResult<()> {
        let mouse_x_ret = self
            .execute(r#"return window.tf_m_mouse_x || -1;"#, Vec::new())
            .await?;
        let mut mouse_x = mouse_x_ret.convert::<i64>()?;

        let mouse_y_ret = self
            .execute(r#"return window.tf_m_mouse_y || -1;"#, Vec::new())
            .await?;
        let mut mouse_y = mouse_y_ret.convert::<i64>()?;

        if mouse_x <= -1 || mouse_y <= -1 {
            self.execute(
                r#"
                window.tf_m_mouse_x = window.tf_m_mouse_x || -1;
                window.tf_m_mouse_y = window.tf_m_mouse_y || -1;

                document.addEventListener("mousemove", (event) => {
                   window.tf_m_mouse_x = event.clientX;
                   window.tf_m_mouse_y = event.clientY;
                });"#,
                Vec::new(),
            )
            .await?;

            self.action_chain().move_by_offset(1, 1).perform().await?;

            let mouse_x_ret = self
                .execute(r#"return window.tf_m_mouse_x || -1;"#, Vec::new())
                .await?;
            mouse_x = mouse_x_ret.convert::<i64>()?;

            let mouse_y_ret = self
                .execute(r#"return window.tf_m_mouse_y || -1;"#, Vec::new())
                .await?;
            mouse_y = mouse_y_ret.convert::<i64>()?;

            if mouse_x <= -1 || mouse_y <= -1 {
                return Err(WebDriverError::CommandRecvError(
                    "Failed to get mouse position".to_string(),
                ));
            }
        }

        let target_rect = target_element.rect().await?;

        let half_width = (target_rect.width / 2.00) as i64;
        let half_height = (target_rect.height / 2.00) as i64;
        let target_pos_x = target_rect.x as i64 + half_width; // Middle of element
        let target_pos_y = target_rect.y as i64 + half_height; // Middle of element

        let quarter_width = half_width / 2;
        let quarter_height = half_height / 2;
        let final_pos_x = target_pos_x + thread_rng().gen_range(-quarter_width..=quarter_width);
        let final_pos_y = target_pos_y + thread_rng().gen_range(-quarter_height..=quarter_height);

        let mut positions = match &action.interpolation {
            MouseInterpolation::Linear => create_linear_steps(
                mouse_x,
                mouse_y,
                final_pos_x,
                final_pos_y,
                action.duration_ms as usize,
            ),
            MouseInterpolation::Spline => create_spline_steps(
                mouse_x,
                mouse_y,
                final_pos_x,
                final_pos_y,
                action.duration_ms as usize,
            ),
        };

        if action.jitter_amount > 0 {
            jitter(&mut positions, action.jitter_amount);
        }

        let action_chain = self.action_chain_with_delay(None, Some(0));
        let mut action_chain = action
            .start_action
            .action(action_chain);

        for point in positions {
            action_chain = action_chain.move_to(point.0, point.1);
        }

        action.end_action.action(action_chain).perform().await?;

        Ok(())
    }
}

impl MouseButtonAction {
    fn action(&self, action_chain: ActionChain) -> ActionChain {
        match self {
            MouseButtonAction::None => action_chain,
            MouseButtonAction::LeftClick => action_chain.click(),
            MouseButtonAction::LeftHold => action_chain.click_and_hold(),
            MouseButtonAction::LeftRelease => action_chain.release(),
            MouseButtonAction::RightClick => action_chain.context_click(),
        }
    }
}

fn jitter(input: &mut [(i64, i64)], amount: i64) {
    input.iter_mut().for_each(|(x, y)| {
        let add_jitter = thread_rng().gen_bool(1.00 / 5.00);
        if add_jitter {
            *x += thread_rng().gen_range(-amount..=amount);
            *y += thread_rng().gen_range(-amount..=amount);
        }
    })
}

fn create_spline_steps(
    start_x: i64,
    start_y: i64,
    end_x: i64,
    end_y: i64,
    steps: usize,
) -> Vec<(i64, i64)> {
    let x_min = start_x.min(end_x);
    let x_max = start_x.max(end_x);
    let y_min = start_y.min(end_y);
    let y_max = start_y.max(end_y);

    let mut rng = thread_rng();
    let x_offset_one = rng.gen_range(x_min..x_max);
    let y_offset_one = rng.gen_range(y_min..y_max);

    let linear_x = Linear::builder()
        .elements([start_x as f64, x_offset_one as f64, end_x as f64])
        .equidistant()
        .normalized()
        .easing(easing::Plateau::new(0.00))
        .build()
        .unwrap();

    let bezier_y = Bezier::builder()
        .elements([start_y as f64, y_offset_one as f64, end_y as f64])
        .normalized::<f64>()
        .constant::<3>()
        .build()
        .unwrap();

    let bspline_y = BSpline::builder()
        .clamped()
        .elements([start_y as f64, y_offset_one as f64, end_y as f64])
        .knots(bezier_y.domain())
        .dynamic()
        .build()
        .unwrap();

    linear_x
        .take(steps)
        .zip(bspline_y.take(steps))
        .map(|(mut x, mut y)| {
            if x.is_sign_negative() {
                x = 0.00;
            }
            if y.is_sign_negative() {
                y = 0.00;
            }
            (x as i64, y as i64)
        })
        .collect::<Vec<_>>()
}

fn create_linear_steps(
    start_x: i64,
    start_y: i64,
    end_x: i64,
    end_y: i64,
    steps: usize,
) -> Vec<(i64, i64)> {
    let linear_x = Linear::builder()
        .elements([start_x as f64, end_x as f64])
        .equidistant::<f64>()
        .normalized()
        .easing(easing::Plateau::new(0.1))
        .build()
        .unwrap();

    let linear_y = Linear::builder()
        .elements([start_y as f64, end_y as f64])
        .equidistant::<f64>()
        .normalized()
        .easing(easing::Plateau::new(0.1))
        .build()
        .unwrap();

    linear_x
        .take(steps)
        .zip(linear_y.take(steps))
        .map(|(mut x, mut y)| {
            if x.is_sign_negative() {
                x = 0.00;
            }
            if y.is_sign_negative() {
                y = 0.00;
            }
            (x as i64, y as i64)
        })
        .collect::<Vec<_>>()
}
