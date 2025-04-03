# Pico Verifiable Computation Service - Tangle Blueprint

This project implements a reusable Tangle Blueprint that provides Verifiable Computation as a Service, powered by the [Pico zkVM](https://github.com/brevis-network/pico).

## Overview

The service allows developers to submit arbitrary RISC-V programs (compiled for the Pico zkVM) along with their inputs and receive Zero-Knowledge Proofs (ZKPs) of the execution trace. This enables trustless verification of computations performed off-chain.

Key Features:

- **Generic Computation:** Supports proving arbitrary RISC-V programs compatible with Pico.
- **On-Chain Program Registry:** Uses an EVM smart contract (`ProgramRegistry.sol`) to store program metadata (SHA256 hash and download location - e.g., IPFS, HTTPS), enabling program reuse and discovery.
- **Flexible Proving:** Supports different proving modes via the Pico SDK:
  - `Fast`: RISC-V execution proof only (for testing/debugging, **not secure**).
  - `Full`: Complete recursive STARK proof generation.
  - `FullWithEvm`: Generates a Groth16 proof verifiable on EVM chains using generated Solidity verifiers.
- **Tangle Blueprint Integration:** Built using the [Tangle Blueprint SDK](https://github.com/TangleLabs/blueprint-sdk), allowing the service to run as a decentralized backend service within the Tangle network ecosystem. Jobs can be triggered via Tangle messages.
- **Decentralized Storage:** Program binaries are intended to be stored off-chain (e.g., IPFS, Arweave, HTTPS), referenced by the on-chain registry.

## Architecture

1.  **Tangle Blueprint Runner (`bin/`):** The main service executable that runs the Blueprint. It listens for incoming job requests (e.g., from the Tangle network).
2.  **Core Logic Library (`lib/`):** Contains the Rust implementation of the service:
    - **Jobs:** Defines the available service functions (`say_hello` example, `generate_proof` core job).
    - **Context:** Manages shared resources like HTTP clients and EVM provider configurations.
    - **EVM Interaction:** Handles communication with the `ProgramRegistry` smart contract using `alloy`.
    - **Program Handling:** Fetches program ELF binaries from specified locations (URL, local path) and verifies their integrity using SHA256 hashes.
    - **Pico Integration:** Uses the `pico-sdk` to load ELFs, provide inputs, and execute the different proving flows (`prove_fast`, `prove`, `prove_evm`).
    - **Types & Errors:** Defines data structures for requests, results, and custom errors.
3.  **Program Registry Contract (`contracts/`):** A Solidity smart contract (`ProgramRegistry.sol`) deployed on an EVM-compatible chain. It stores `programHash -> {location, owner}` mappings.
4.  **Program Storage (External):** A separate system (e.g., IPFS, web server) hosts the actual program ELF binaries.

## Workflow

1.  **Program Registration (Developer):**
    - Compile RISC-V code to a Pico zkVM ELF binary.
    - Calculate the SHA256 hash of the ELF file.
    - Upload the ELF file to a persistent storage location (e.g., IPFS) and get its URI/URL.
    - Call the `registerProgram` function on the `ProgramRegistry` smart contract with the program hash and location URI.
2.  **Proof Request (User/Application):**
    - Construct a `ProofRequest` containing:
      - `program_hash`: The hash of the registered program to execute.
      - `inputs`: Hex-encoded input data for the program.
      - `proving_type`: `Fast`, `Full`, or `FullWithEvm`.
      - Optional overrides for program location or EVM configuration.
    - Submit the `ProofRequest` as a job call to the running Tangle Blueprint service (e.g., via a Tangle message targeting the service ID and `GENERATE_PROOF_JOB_ID`).
3.  **Proof Generation (Service):**
    - The Blueprint Runner receives the job call.
    - The `generate_proof` job function executes:
      - Retrieves the program location from the `ProgramRegistry` contract (unless overridden).
      - Downloads the ELF binary from the location.
      - Verifies the downloaded ELF hash against the requested `program_hash`.
      - Initializes the Pico `DefaultProverClient` (KoalaBear).
      - Executes the requested proving type (`prove_fast`, `prove`, or `prove_evm`).
      - For `prove_evm`, runs external Docker commands via the SDK to generate the final Groth16 proof and EVM verifier inputs.
      - Parses and collects the proof data and public values.
    - Returns the `ProofResult` (containing proof data, public values, etc.) back through the Blueprint SDK (e.g., as a response Tangle message).

## Setup & Usage

_(TODO: Add instructions on how to build, configure (environment variables for RPC URL, registry address, etc.), deploy the contract, and run the blueprint service.)_

## Development

_(TODO: Add details on building the code, running tests, contributing guidelines.)_
