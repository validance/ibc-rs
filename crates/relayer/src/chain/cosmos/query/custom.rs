use crate::chain::requests::CrossChainQueryRequest;
use crate::error::Error;
use tendermint_rpc::{Client, HttpClient};
use ibc_relayer_types::applications::ics31_icq::{
    response::CrossChainQueryResponse,
    error::Error as CrossChainQueryError
};
use hex;
use ibc_proto::ibc::core::commitment::v1::MerkleProof;
use ibc_relayer_types::core::ics23_commitment::merkle::convert_tm_to_ics_merkle_proof;

pub async fn cross_chain_query_via_rpc(
    client: &HttpClient,
    cross_chain_query_request: CrossChainQueryRequest,
) -> Result<CrossChainQueryResponse, Error> {
    let hex_decoded_request = hex::decode(cross_chain_query_request.request).map_err(|_| Error::ics31(CrossChainQueryError::parse()))?;

    let response = client.abci_query(
        Some(cross_chain_query_request.query_type),
        hex_decoded_request,
        Some(cross_chain_query_request.height),
        true,
    ).await.map_err(|_| Error::ics31(CrossChainQueryError::query()))?;

    if !response.code.is_ok() {
        return Err(Error::ics31(CrossChainQueryError::query()));
    }

    if response.proof.is_none() {
        return Err(Error::ics31(CrossChainQueryError::proof()));
    }

    let proof = response
        .proof
        .map(|p| convert_tm_to_ics_merkle_proof(&p))
        .transpose()
        .map_err(Error::ics23)?
        .ok_or_else(|| MerkleProof{proofs: vec![]})
        .map_err(|_| Error::ics31(CrossChainQueryError::proof()))?;


    Ok(
        CrossChainQueryResponse::new(
            cross_chain_query_request.chain_id.to_string(),
            cross_chain_query_request.query_id,
            hex::encode(response.value),
            response.height.to_string(),
            proof,
        )
    )
}