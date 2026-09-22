//! A small event log. Later phases replace this with a bus that can wake a
//! goal without a user prompt.

use raven_core::Phase;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub name: String,
    pub detail: String,
    pub phase: Option<Phase>,
}

#[derive(Clone, Debug, Default)]
pub struct EventLog {
    events: Vec<Event>,
}

impl EventLog {
    pub fn publish(
        &mut self,
        name: impl Into<String>,
        detail: impl Into<String>,
        phase: Option<Phase>,
    ) {
        self.events.push(Event {
            name: name.into(),
            detail: detail.into(),
            phase,
        });
    }

    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_in_order() {
        let mut log = EventLog::default();
        log.publish("goal.created", "prepare", Some(Phase::Perceive));
        log.publish("tool.done", "calendar", Some(Phase::Act));
        assert_eq!(log.len(), 2);
        assert_eq!(log.iter().next().unwrap().name, "goal.created");
    }
}
