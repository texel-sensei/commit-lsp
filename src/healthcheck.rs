//! Functionality to check whether the environment is sane and reporting this to the user.

use std::error::Error;

use colored::Colorize as _;

pub struct HealthReport {
    context: String,
    is_silent: bool,
}

impl HealthReport {
    pub fn new(ctx: impl Into<String>) -> Self {
        Self {
            context: ctx.into(),
            is_silent: false,
        }
    }

    pub fn silent() -> Self {
        Self {
            context: String::new(),
            is_silent: true,
        }
    }

    pub fn set_context(&mut self, ctx: impl Into<String>) {
        self.context = ctx.into();

        if self.is_silent {
            return;
        }

        let len = self.context.len() + 2;

        let padding = "-".repeat((80 - len) / 2);

        println!("{} {} {}", padding, self.context.bold(), padding);
    }

    pub fn report(&mut self, name: impl Into<String>, state: ComponentState) {
        if self.is_silent {
            return;
        }

        print!("\n- {}: ", name.into());
        match state {
            ComponentState::Ok(None) => println!("{}", "OK".green()),
            ComponentState::Ok(Some(txt)) => println!("{}\n    {}", "OK".green(), txt),
            ComponentState::Info(txt) => println!("{}\n    {}", "INFO".blue(), txt),
            ComponentState::Warning(txt) => println!("{}\n    {}", "WARNING".yellow(), txt),
            ComponentState::Error(txt) => println!("{}\n    {}", "ERROR".red(), txt),
        }
    }

    pub fn start(&mut self, name: impl Into<String>) -> OngoingReport<'_> {
        OngoingReport {
            health: self,
            component: name.into(),
        }
    }
}

pub enum ComponentState {
    Ok(Option<String>),
    Info(String),
    Warning(String),
    Error(String),
}

pub trait ResultExt: Sized {
    fn report(self, report: &mut HealthReport, name: impl Into<String>) -> Self {
        let check = report.start(name);

        self.finish_check(check)
    }

    fn finish_check<'a>(self, check: OngoingReport<'a>) -> Self;
}

impl<V, E> ResultExt for Result<V, E>
where
    E: Error,
{
    fn finish_check<'a>(self, check: OngoingReport<'a>) -> Self {
        match &self {
            Ok(_) => check.ok(),
            Err(e) => check.error(e.to_string()),
        }
        self
    }
}

impl<T> ResultExt for Option<T> {
    fn finish_check<'a>(self, check: OngoingReport<'a>) -> Self {
        match &self {
            Some(_) => check.ok(),
            None => check.error(""),
        }

        self
    }
}

#[must_use]
pub struct OngoingReport<'a> {
    health: &'a mut HealthReport,
    component: String,
}

impl<'a> OngoingReport<'a> {
    pub fn complete(self, state: ComponentState) {
        self.health.report(self.component, state);
    }

    pub fn ok(self) {
        self.health.report(self.component, ComponentState::Ok(None));
    }

    pub fn ok_with(self, txt: impl Into<String>) {
        self.health.report(self.component, ComponentState::Ok(Some(txt.into())));
    }

    pub fn info(self, txt: impl Into<String>) {
        self.health
            .report(self.component, ComponentState::Info(txt.into()));
    }

    pub fn warn(self, txt: impl Into<String>) {
        self.health
            .report(self.component, ComponentState::Warning(txt.into()));
    }

    pub fn error(self, txt: impl Into<String>) {
        self.health
            .report(self.component, ComponentState::Error(txt.into()));
    }
}
