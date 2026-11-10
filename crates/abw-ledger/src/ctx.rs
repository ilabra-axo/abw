//! Who is asking, for which tenant, under which correlation (target-system-design §3.1.4
//! envelope columns `actor`, `actor_kind`, `correlation_id`; §3.8 `CommandCtx`).

use uuid::Uuid;

/// `ledger.events.actor_kind` (§3.1.4 CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorKind {
    Human,
    Agent,
    Platform,
    Tool,
}

impl ActorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActorKind::Human => "human",
            ActorKind::Agent => "agent",
            ActorKind::Platform => "platform",
            ActorKind::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub id: String,
    pub kind: ActorKind,
}

impl Actor {
    pub fn human(id: impl Into<String>) -> Actor {
        Actor {
            id: id.into(),
            kind: ActorKind::Human,
        }
    }

    pub fn agent(id: impl Into<String>) -> Actor {
        Actor {
            id: id.into(),
            kind: ActorKind::Agent,
        }
    }

    pub fn platform(id: impl Into<String>) -> Actor {
        Actor {
            id: id.into(),
            kind: ActorKind::Platform,
        }
    }

    pub fn tool(id: impl Into<String>) -> Actor {
        Actor {
            id: id.into(),
            kind: ActorKind::Tool,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandCtx {
    pub tenant: String,
    pub actor: Actor,
    pub correlation_id: Uuid,
}

impl CommandCtx {
    /// A context with a fresh correlation id.
    pub fn new(tenant: impl Into<String>, actor: Actor) -> CommandCtx {
        CommandCtx {
            tenant: tenant.into(),
            actor,
            correlation_id: Uuid::new_v4(),
        }
    }

    pub fn with_correlation(mut self, correlation_id: Uuid) -> CommandCtx {
        self.correlation_id = correlation_id;
        self
    }
}
