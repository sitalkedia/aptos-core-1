// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_config::{
    AccountResource, BalanceResource, CRSNResource, ChainIdResource, DiemAccountResource,
};
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::{
        access_path_for_config, dpn_access_path_for_config, ConfigurationResource, OnChainConfig,
        ValidatorSet,
    },
    state_store::state_key::StateKey,
};
use move_core_types::{account_address::AccountAddress, move_resource::MoveResource};
use serde::de::DeserializeOwned;

pub struct AccountStateView<'a, F>
where
    F: Fn(&StateKey) -> anyhow::Result<Option<Vec<u8>>>,
{
    pub account_address: &'a AccountAddress,
    pub state_value_resolver: F,
}

impl<'a, F> AccountStateView<'a, F>
where
    F: Fn(&StateKey) -> anyhow::Result<Option<Vec<u8>>>,
{
    pub fn get_validator_set(&self) -> anyhow::Result<Option<ValidatorSet>> {
        self.get_config::<ValidatorSet>()
    }

    pub fn get_configuration_resource(&self) -> anyhow::Result<Option<ConfigurationResource>> {
        self.get_move_resource::<ConfigurationResource>()
    }

    fn get_move_resource<T: MoveResource>(&self) -> anyhow::Result<Option<T>> {
        let state_key = self.get_state_key_for_path(T::struct_tag().access_vector());
        self.get_resource_impl(&state_key)
    }

    fn get_config<T: OnChainConfig>(&self) -> anyhow::Result<Option<T>> {
        let state_key = self.get_state_key_for_path(access_path_for_config(T::CONFIG_ID).path);

        match self.get_resource_impl(&state_key)? {
            Some(config) => Ok(Some(config)),
            _ => self.get_resource_impl(
                &self.get_state_key_for_path(dpn_access_path_for_config(T::CONFIG_ID).path),
            ),
        }
    }

    pub fn get_resource<T: MoveResource>(&self) -> anyhow::Result<Option<T>> {
        self.get_resource_impl(&self.get_state_key_for_path(T::struct_tag().access_vector()))
    }

    pub fn get_chain_id_resource(&self) -> anyhow::Result<Option<ChainIdResource>> {
        self.get_resource::<ChainIdResource>()
    }

    pub fn get_crsn_resource(&self) -> anyhow::Result<Option<CRSNResource>> {
        self.get_resource::<CRSNResource>()
    }

    pub fn get_balance_resources(&self) -> anyhow::Result<Option<BalanceResource>> {
        self.get_resource::<BalanceResource>()
    }

    fn get_state_key_for_path(&self, path: Vec<u8>) -> StateKey {
        StateKey::AccessPath(AccessPath::new(*self.account_address, path))
    }

    // Return the `AccountResource` for this blob. If the blob doesn't have an `AccountResource`
    // then it must have a `DiemAccountResource` in which case we convert that to an
    // `AccountResource`.
    pub fn get_account_resource(&self) -> anyhow::Result<Option<AccountResource>> {
        match self.get_resource::<AccountResource>()? {
            x @ Some(_) => Ok(x),
            None => match self.get_resource::<DiemAccountResource>()? {
                Some(diem_ar) => Ok(Some(AccountResource::new(
                    diem_ar.sequence_number(),
                    diem_ar.authentication_key().to_vec(),
                    diem_ar.address(),
                ))),
                None => Ok(None),
            },
        }
    }

    fn get_resource_impl<T: DeserializeOwned>(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<T>> {
        (self.state_value_resolver)(state_key)?
            .map(|bytes| bcs::from_bytes(&bytes))
            .transpose()
            .map_err(Into::into)
    }
}
