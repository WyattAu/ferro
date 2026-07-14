//! Chaos engineering framework for Ferro
//!
//! Inspired by Netflix's Chaos Monkey, this crate provides fault injection
//! capabilities for testing system resilience.

pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;
pub mod orchestrator;

pub use orchestrator::{ChaosExperiment, ChaosOrchestrator};
