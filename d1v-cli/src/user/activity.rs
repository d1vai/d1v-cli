use std::borrow::Cow;
use std::sync::LazyLock;

use colorgrad::{Gradient, GradientBuilder, LinearGradient};
use d1v_api::{DailyCount, PromptDailyActivity};

use super::{ActivityArgs, ActivityTarget};
use crate::error::Result;
use crate::text::{Field, Fields, Line, Render, RenderContext, Span};
use crate::Context;
use crate::{t, theme};

const BAR_WIDTH: usize = 20;

static BAR_GRADIENT: LazyLock<LinearGradient> = LazyLock::new(|| {
    GradientBuilder::new()
        .html_colors(&["#4A9EFF", "#36D399"])
        .build::<LinearGradient>()
        .expect("valid gradient colors")
});

struct ActivityView<'a>(&'a PromptDailyActivity);

impl Render for ActivityView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        let activity = self.0;

        Fields::new([
            field_row(
                t!("activity-label-period"),
                format!("{} ~ {}", activity.start_date, activity.end_date),
            ),
            field_row(t!("activity-label-days"), activity.days.to_string()),
        ])
        .render(ctx)?;

        if activity.counts.is_empty() {
            return Ok(());
        }

        writeln!(ctx.writer)?;
        let max = activity
            .counts
            .iter()
            .map(|entry| entry.count)
            .max()
            .unwrap_or(1)
            .max(1);

        for entry in &activity.counts {
            bar_line(entry, max).render(ctx)?;
            writeln!(ctx.writer)?;
        }

        Ok(())
    }
}

fn field_row(label: String, value: impl Into<Cow<'static, str>>) -> Field {
    Field::new(
        Span::styled(label, theme::ansi::label()),
        Span::styled(value, theme::ansi::value()),
    )
}

fn bar_line(entry: &DailyCount, max: i32) -> Line {
    let filled =
        ((entry.count as f32 / max as f32 * BAR_WIDTH as f32).round() as usize).min(BAR_WIDTH);
    let empty = BAR_WIDTH - filled;

    let mut line = Line::raw("  ")
        .push_styled(entry.date.clone(), theme::ansi::dim())
        .push_plain(" ");

    for color in BAR_GRADIENT.colors(BAR_WIDTH).into_iter().take(filled) {
        let [r, g, b, _] = color.to_rgba8();
        line = line.push_styled("█", theme::ansi::rgb(r, g, b));
    }

    if empty > 0 {
        line = line.push_styled("░".repeat(empty), theme::ansi::rgb(60, 60, 60));
    }

    line.push_plain(" ")
        .push_styled(entry.count.to_string(), theme::ansi::label())
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

    ctx.present(ActivityView(&activity), &activity)
}
