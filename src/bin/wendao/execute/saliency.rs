//! Saliency command execution.

use crate::helpers::emit;
use crate::types::{Cli, Command, SaliencyCommand};
use anyhow::Result;
use serde_json::json;
use xiuxian_wendao::{LinkGraphSaliencyTouchRequest, valkey_saliency_get, valkey_saliency_touch};

pub(super) fn handle(cli: &Cli) -> Result<()> {
    let Command::Saliency { command } = &cli.command else {
        unreachable!("saliency handler must be called with saliency command");
    };

    match command {
        SaliencyCommand::Get { node_id } => {
            let payload = valkey_saliency_get(node_id).map_err(anyhow::Error::msg)?;
            emit(&json!({"node_id": node_id, "state": payload}), cli.output)
        }
        SaliencyCommand::Touch {
            node_id,
            activation_delta,
            saliency_base,
            decay_rate,
            alpha,
            minimum_saliency,
            maximum_saliency,
            now_unix,
        } => {
            let state = valkey_saliency_touch(LinkGraphSaliencyTouchRequest {
                node_id: node_id.clone(),
                activation_delta: *activation_delta,
                saliency_base: *saliency_base,
                decay_rate: *decay_rate,
                alpha: *alpha,
                minimum_saliency: *minimum_saliency,
                maximum_saliency: *maximum_saliency,
                now_unix: *now_unix,
            })
            .map_err(anyhow::Error::msg)?;
            emit(&state, cli.output)
        }
    }
}
