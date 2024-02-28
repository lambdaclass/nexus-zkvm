use std::{
    io,
    path::{Path, PathBuf},
};

use anyhow::Context;

use nexus_config::{vm as vm_config, Config};
use nexus_prover::error::ProofError;
use nexus_tools_dev::command::common::public_params::{
    format_params_file, PublicParamsAction, PublicParamsArgs, SetupArgs,
};

use crate::{command::cache_path, LOG_TARGET};

pub fn handle_command(args: PublicParamsArgs) -> anyhow::Result<()> {
    let action = args
        .command
        .unwrap_or_else(|| PublicParamsAction::Setup(SetupArgs::default()));
    match action {
        PublicParamsAction::Setup(setup_args) => {
            let _ = setup_params(setup_args)?;
        }
    }
    Ok(())
}

pub(crate) fn setup_params(args: SetupArgs) -> anyhow::Result<PathBuf> {
    let vm_config = vm_config::VmConfig::from_env()?;

    let force = args.force;
    let k = args.k.unwrap_or(vm_config.k);
    let nova_impl = args.nova_impl.unwrap_or(vm_config.nova_impl);
    let srs_file = args.srs_file.as_deref();

    let path = match args.path {
        Some(path) => path,
        None => {
            let pp_file_name = format_params_file(nova_impl, k);
            let cache_path = cache_path()?;

            cache_path.join(pp_file_name)
        }
    };

    if !force && path.try_exists()? {
        tracing::info!(
            target: LOG_TARGET,
            "path {} already exists, use `setup --force` to overwrite",
            path.display(),
        );
        return Ok(path);
    }

    setup_params_to_file(&path, nova_impl, k, srs_file)?;
    Ok(path)
}

fn setup_params_to_file(
    path: &Path,
    nova_impl: vm_config::NovaImpl,
    k: usize,
    srs_file: Option<&Path>,
) -> anyhow::Result<()> {
    let path = path.to_str().context("path is not valid utf8")?;
    match nova_impl {
        vm_config::NovaImpl::Sequential => {
            nexus_prover::pp::gen_to_file(k, false, false, path, None)
        }
        vm_config::NovaImpl::Parallel => nexus_prover::pp::gen_to_file(k, true, false, path, None),
        vm_config::NovaImpl::ParallelCompressible => match srs_file {
            None => {
                tracing::error!(
                    target: LOG_TARGET,
                    "SRS file is required for parallel compressible proofs"
                );
                return Err(ProofError::MissingSRS.into());
            }
            Some(srs_file) => {
                if !srs_file.try_exists()? {
                    tracing::error!(
                        target: LOG_TARGET,
                        "path {} was not found",
                        srs_file.display(),
                    );
                    return Err(io::Error::from(io::ErrorKind::NotFound).into());
                }
                let srs_file_str = srs_file.to_str().context("path is not valid utf8")?;
                nexus_prover::pp::gen_to_file(k, true, true, path, Some(srs_file_str))
            }
        },
    };
    Ok(())
}
