// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/Strings.sol";

/**
 * @title ProgramRegistry
 * @notice Stores metadata for verifiable computation programs (e.g., Pico zkVM ELF binaries).
 * @dev Stores program hash (SHA256) and a location hint (URL, IPFS CID) for off-chain fetching.
 * Uses Ownable for contract administration and allows program owners to update their entries.
 */
contract ProgramRegistry is Ownable {
    using Strings for bytes32; // Optional: If you need to convert hash to string

    struct ProgramInfo {
        string location; // URL, IPFS CID (e.g., "ipfs://Qm...", "https://...")
        address owner;   // Address that registered/owns the program entry
        bool exists;     // Flag to check existence, prevents updating non-existent entries implicitly
    }

    mapping(bytes32 => ProgramInfo) public programs;

    // --- Events ---

    /**
     * @notice Emitted when a new program is successfully registered.
     * @param programHash The SHA256 hash of the program binary.
     * @param location The location hint (URL, IPFS CID) for fetching the program.
     * @param owner The address that registered the program.
     */
    event ProgramRegistered(bytes32 indexed programHash, string location, address indexed owner);

    /**
     * @notice Emitted when an existing program's location is updated.
     * @param programHash The SHA256 hash of the program binary being updated.
     * @param newLocation The updated location hint.
     * @param updater The address that performed the update (must be the program owner).
     */
    event ProgramUpdated(bytes32 indexed programHash, string newLocation, address indexed updater);

    /**
     * @notice Emitted when a program entry is transferred to a new owner.
     * @param programHash The SHA256 hash of the program binary.
     * @param newOwner The address that became the new owner.
     */
    event ProgramEntryTransferred(bytes32 indexed programHash, address indexed newOwner);

    // --- Errors --- // Consider using Custom Errors for gas savings
    error ProgramRegistry__LocationCannotBeEmpty();
    error ProgramRegistry__ProgramNotFound();
    error ProgramRegistry__NotProgramOwner();
    error ProgramRegistry__ProgramAlreadyExists(); // For explicit registration function

    // --- Constructor ---

    /**
     * @notice Sets the deployer as the initial contract owner.
     */
    constructor() Ownable(msg.sender) {} // Pass initial owner to Ownable constructor

    // --- Registration and Updates ---

    /**
     * @notice Registers a new program. Reverts if the program hash is already registered.
     * @dev Sets the caller (`msg.sender`) as the owner of this program entry.
     * @param _programHash SHA256 hash of the program binary.
     * @param _location String indicating where to download the program (URL, IPFS CID). Must not be empty.
     */
    function registerProgram(bytes32 _programHash, string calldata _location) external {
        if (bytes(_location).length == 0) {
            revert ProgramRegistry__LocationCannotBeEmpty();
        }
        ProgramInfo storage info = programs[_programHash];
        if (info.exists) {
            revert ProgramRegistry__ProgramAlreadyExists();
        }

        programs[_programHash] = ProgramInfo({
            location: _location,
            owner: msg.sender,
            exists: true
        });

        emit ProgramRegistered(_programHash, _location, msg.sender);
    }

    /**
     * @notice Updates the location of an already registered program.
     * @dev Only the current owner of the program entry (`ProgramInfo.owner`) can call this.
     * @param _programHash SHA256 hash of the program binary to update.
     * @param _newLocation The new location string. Must not be empty.
     */
    function updateProgramLocation(bytes32 _programHash, string calldata _newLocation) external {
         if (bytes(_newLocation).length == 0) {
            revert ProgramRegistry__LocationCannotBeEmpty();
        }
        ProgramInfo storage info = programs[_programHash];
        if (!info.exists) {
             revert ProgramRegistry__ProgramNotFound();
        }
        if (msg.sender != info.owner) {
             revert ProgramRegistry__NotProgramOwner();
        }

        info.location = _newLocation;
        emit ProgramUpdated(_programHash, _newLocation, msg.sender);
    }

     /**
     * @notice Allows the owner of a program entry to transfer ownership of that specific entry.
     * @dev This is different from contract ownership managed by Ownable.
     * @param _programHash SHA256 hash of the program entry to transfer.
     * @param _newProgramOwner The address to transfer ownership of the program entry to. Cannot be the zero address.
     */
    function transferProgramEntryOwnership(bytes32 _programHash, address _newProgramOwner) external {
        require(_newProgramOwner != address(0), "ProgramRegistry: New program owner cannot be zero address");
        ProgramInfo storage info = programs[_programHash];
         if (!info.exists) {
             revert ProgramRegistry__ProgramNotFound();
        }
         if (msg.sender != info.owner) {
             revert ProgramRegistry__NotProgramOwner();
        }

        info.owner = _newProgramOwner;
        emit ProgramEntryTransferred(_programHash, _newProgramOwner);
    }

    // --- Views ---

    /**
     * @notice Get the program information for a registered program.
     * @param _programHash SHA256 hash of the program binary.
     * @return info The ProgramInfo struct containing location and owner. Reverts if not found.
     */
    function getProgramInfo(bytes32 _programHash) external view returns (ProgramInfo memory info) {
        info = programs[_programHash];
        if (!info.exists) {
            revert ProgramRegistry__ProgramNotFound();
        }
    }

     /**
     * @notice Get the location string for a registered program.
     * @param _programHash SHA256 hash of the program binary.
     * @return location String indicating where to download the program. Reverts if not found.
     */
    function getProgramLocation(bytes32 _programHash) external view returns (string memory location) {
        ProgramInfo storage info = programs[_programHash];
         if (!info.exists) {
             revert ProgramRegistry__ProgramNotFound();
        }
        return info.location;
    }

     /**
     * @notice Check if a program hash is registered.
     * @param _programHash SHA256 hash of the program binary.
     * @return True if registered, false otherwise.
     */
    function isRegistered(bytes32 _programHash) external view returns (bool) {
        return programs[_programHash].exists;
    }

     /**
     * @notice Get the owner of a specific program entry.
     * @param _programHash SHA256 hash of the program binary.
     * @return owner The address owning the program entry. Reverts if not found.
     */
    function getProgramOwner(bytes32 _programHash) external view returns (address owner) {
        ProgramInfo storage info = programs[_programHash];
         if (!info.exists) {
             revert ProgramRegistry__ProgramNotFound();
        }
        return info.owner;
    }
}
