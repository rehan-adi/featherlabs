use crate::*;

pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateAsset<'info>>,
    lrp: LightRootParams,
    seeds: u64,
    args: CreateAssetArgsV1,
) -> Result<()> {
    let remaining_accounts = ctx.remaining_accounts;
    let mut ctx: LightContext<CreateAsset, LightCreateAsset> = LightContext::new(
        ctx,
        lrp.inputs,
        lrp.merkle_context,
        lrp.merkle_tree_root_index,
        lrp.address_merkle_context,
        lrp.address_merkle_tree_root_index,
    )?;
    // let seeds = match args.asset_type {
    //     AssetType::Alone { seeds } => {
    //         let complete_seed:&[&[u8]] = &[ASSET_SEED, seeds.to_le_bytes().as_ref()];
    //         complete_seed
    //     },
    //     AssetType::Member { group_seeds } => {
    //         let group:LightMutAccount<GroupV1> = LightMutAccount::try_from_slice(lrp.inputs., merkle_context, merkle_tree_root_index, address_merkle_context)
    //         let complete_seed:&[&[u8]] = &[ASSET_SEED, seeds.to_le_bytes().as_ref()];
    //     }
    // };
    let inputs = &ParamsCreateAsset { seeds };
    ctx.check_constraints(inputs)?;
    ctx.derive_address_seeds(lrp.address_merkle_context, inputs);
    let asset = &mut ctx.light_accounts.asset;
    let asset_address_param = &mut asset.new_address_params().unwrap();
    let mut output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext> = vec![];
    let mut new_address_params = vec![asset_address_param.clone()];
    let address_merkle_context =
        unpack_address_merkle_context(lrp.address_merkle_context, remaining_accounts);
    let asset_address = Pubkey::new_from_array(derive_address(
        &asset_address_param.seed,
        &address_merkle_context,
    ));
    asset.address = asset_address;
    asset.has_multisig = false;
    asset.asset_authority_state = AssetAuthorityVariantV1::Owner;
    asset.asset_state = AssetStateV1::Unlocked;
    asset.group_membership = None;
    asset.rentable = args.rentable;
    asset.transferable = args.transferrable;
    match args.metadata {
        Some(metadata) => {
            asset.has_metadata = true;
            let mut acc: LightInitAccount<AssetDataV1> = LightInitAccount::new(
                &lrp.merkle_context,
                &lrp.address_merkle_context,
                lrp.address_merkle_tree_root_index,
            );
            let address_seed = derive_address_seed(
                &[ASSET_DATA_SEED, asset_address.as_ref()],
                &crate::ID,
                &address_merkle_context,
            );
            acc.set_address_seed(address_seed);
            new_address_params.push(acc.new_address_params());
            acc.asset_key = asset_address;
            acc.attributes = metadata.attributes;
            acc.mutable = metadata.mutable;
            acc.name = metadata.name;
            acc.uri = metadata.uri;
            acc.privilege_attributes = Vec::new();
            let compressed = acc.output_compressed_account(&crate::ID, remaining_accounts)?;
            output_compressed_accounts.push(compressed);
        }
        None => asset.has_metadata = false,
    }
    match args.royalty {
        Some(royalty) => {
            asset.has_royalties = true;
            let mut acc: LightInitAccount<AssetRoyaltiesV1> = LightInitAccount::new(
                &lrp.merkle_context,
                &lrp.address_merkle_context,
                lrp.address_merkle_tree_root_index,
            );
            let address_seed = derive_address_seed(
                &[ASSET_ROYALTY_SEED, asset_address.as_ref()],
                &crate::ID,
                &address_merkle_context,
            );
            acc.set_address_seed(address_seed);
            new_address_params.push(acc.new_address_params());
            acc.asset_key = asset_address;
            acc.basis_points = royalty.basis_points;
            acc.creators = royalty.creators;
            acc.ruleset = royalty.ruleset;
            let compressed = acc.output_compressed_account(&crate::ID, remaining_accounts)?;
            output_compressed_accounts.push(compressed);
        }
        None => asset.has_royalties = false,
    }
    output_compressed_accounts.push(
        asset
            .output_compressed_account(&crate::ID, remaining_accounts)?
            .ok_or(FeatherErrorCode::CustomError)?,
    );
    let bump = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        ctx.accounts.get_invoking_program().key,
    )
    .1;
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    let instruction = InstructionDataInvokeCpi {
        proof: Some(lrp.proof),
        new_address_params,
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: None,
    };
    verify(&ctx, &instruction, &[signer_seeds.as_slice()])?;
    Ok(())
}
#[light_accounts]
#[instruction(seeds: u64)]
pub struct CreateAsset<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    /// CHECK: this is safe
    pub authority: UncheckedAccount<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::FeatherAssets>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    #[light_account(init, seeds = [ASSET_SEED, authority.key().as_ref(), seeds.to_le_bytes().as_ref()])]
    pub asset: LightAccount<AssetV1>,
}

struct ParamsCreateAsset {
    pub seeds: u64,
}
