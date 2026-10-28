/// One step of the update pipeline, in execution order.
///
/// `--only` selects an explicit subset; `Config::runs` decides which steps run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Pull,
    Flake,
    Stage,
    Rebuild,
    Cleanup,
    Diff,
}

impl Step {
    /// Every step, in execution order.
    pub(crate) const ALL: [Step; 6] = [
        Step::Pull,
        Step::Flake,
        Step::Stage,
        Step::Rebuild,
        Step::Cleanup,
        Step::Diff,
    ];

    /// CLI name of the step (`--only <name>`).
    pub(crate) fn name(self) -> &'static str {
        match self {
            Step::Pull => "pull",
            Step::Flake => "flake",
            Step::Stage => "stage",
            Step::Rebuild => "rebuild",
            Step::Cleanup => "cleanup",
            Step::Diff => "diff",
        }
    }

    /// Look a step up by its CLI name. `update` is an alias for `flake`, to
    /// match the `--update` flag people already know.
    pub(crate) fn from_name(name: &str) -> Option<Step> {
        if name == "update" {
            return Some(Step::Flake);
        }
        Step::ALL.into_iter().find(|step| step.name() == name)
    }

    /// Comma-separated CLI names, for error messages.
    pub(crate) fn names() -> String {
        Step::ALL
            .iter()
            .map(|step| step.name())
            .collect::<Vec<_>>()
            .join(",")
    }
}
