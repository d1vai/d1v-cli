use colorgrad::{Gradient, GradientBuilder, LinearGradient};
use d1v_api::PromptDailyActivity;
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

use super::{write_row, ActivityArgs, ActivityTarget, LABEL_WIDTH};
use crate::error::Result;
use crate::output::pad_label;
use crate::Context;
use crate::{t, theme};

const BAR_WIDTH: usize = 20;

#[derive(Serialize)]
#[serde(transparent)]
struct ActivityDisplay<'a>(&'a PromptDailyActivity);

static BAR_GRADIENT: LazyLock<LinearGradient> = LazyLock::new(|| {
    GradientBuilder::new()
        .html_colors(&["#4A9EFF", "#36D399"])
        .build::<LinearGradient>()
        .expect("valid gradient colors")
});

impl Display for ActivityDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let activity = self.0;

        write!(
            f,
            "{}{}",
            pad_label(t!("activity-label-period"), LABEL_WIDTH)
                .if_supports_color(Stream::Stdout, |s| s.style(theme::owo::label())),
            format!("{} ~ {}", activity.start_date, activity.end_date)
                .if_supports_color(Stream::Stdout, |s| s.style(theme::owo::value())),
        )?;
        write_row(f, "activity-label-days", &activity.days.to_string())?;

        if activity.counts.is_empty() {
            return Ok(());
        }

        let max_count = activity
            .counts
            .iter()
            .map(|c| c.count)
            .max()
            .unwrap_or(1)
            .max(1);
        let bar_colors = BAR_GRADIENT.colors(BAR_WIDTH);

        writeln!(f)?;
        for entry in &activity.counts {
            let ratio = entry.count as f32 / max_count as f32;
            let filled = (ratio * BAR_WIDTH as f32).round() as usize;
            let empty = BAR_WIDTH - filled;

            let filled_bar: String = bar_colors[..filled]
                .iter()
                .map(|c| {
                    let [r, g, b, _] = c.to_rgba8();
                    '█'
                        .if_supports_color(Stream::Stdout, |s| s.truecolor(r, g, b))
                        .to_string()
                })
                .collect();
            let empty_bar = "░"
                .repeat(empty)
                .if_supports_color(Stream::Stdout, |s| s.truecolor(60, 60, 60))
                .to_string();

            write!(
                f,
                "\n  {} {}{} {}",
                entry
                    .date
                    .if_supports_color(Stream::Stdout, |s| s.style(theme::owo::dim())),
                filled_bar,
                empty_bar,
                entry
                    .count
                    .if_supports_color(Stream::Stdout, |s| s.style(theme::owo::label())),
            )?;
        }

        Ok(())
    }
}

pub async fn run(ctx: &Context, args: ActivityArgs) -> Result<()> {
    let days = args.days;
    let api = ctx.client.user();

    let activity = match args.target {
        Some(ActivityTarget::User { user_id }) => {
            api.prompt_daily_activity_by_user(user_id, days).await?
        }
        Some(ActivityTarget::Slug { slug }) => {
            api.prompt_daily_activity_by_slug(&slug, days).await?
        }
        None => api.prompt_daily_activity(days).await?,
    };

    ctx.print(&ActivityDisplay(&activity))
}
