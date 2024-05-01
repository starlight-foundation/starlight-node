# Node Architecture 

## Processes
The Starlight node architecture consists of several components, called processes, each with its own thread, that work together to enable the functioning of the Starlight blockchain network. Each process can be referenced by a `Handle`, and has a `Mailbox` to receive messages. For efficient message passing, Tokio channels are used behind the scenes. The different processes are:

1. `Scheduler`: This process manages the timing of operations in the Starlight network by controlling when the node switches between different modes based on a schedule. Starlight divides time into slots, and each slot has a designated leader node responsible for creating new blocks. The `Scheduler`'s main job is to track this leader schedule and tell the node when to enter and exit leader mode. When in leader mode, the node creates new blocks.
- Sends:
  - *Start of Leader Mode*: Sent at the start of the slot before the node's first leader slot, telling the node to start accepting transactions from the network. 
  - *End of Leader Mode*: Sent at the end of the node's last leader slot, telling the node to stop accepting transactions.
  - *New Leader Slot*: Sent at the start of each of the node's leader slots, triggering the node to process queued transactions, create a new block, and send it out to the network.

2. `Transmitter`: This process handles the send half of our UDP socket. It maintains the telemetry protocol state, and broadcasts to other nodes.
- Receives:
  - *Telemetry Note*: When the `Receiver` encounters a telemetry message over the wire, it will send it to the `Transmitter` for processing.
  - *Shred Note*: When the `Assembler` needs a shred to be broadcasted to other nodes, or when the `Ledger` needs a newly minted shred to be sent throughout the network, they will send this message, and the `Transmitter` will send the note over the network to a subset of its peers.
  - *Shred Request*: TODO

3. `Receiver`: This process handles the receive half of the UDP socket, and accepts incoming messages from the network. 
- Sends:
  - *Transaction*: Forwards incoming transactions to a random `TxPool` for further processing.
  - *Open*: Forwards incoming open requests to the `OpenPool` for further processing.
  - *Shred Note*: Forwards incoming block shreds to the `Assembler` for assembly.
  - *Telemetry Note*: Forwards incoming telemetry messages to the `Transmitter` for processing
  - *Vote*: Forwards incoming votes to the `VotePool` for further processing.
  - *Shred Request*: Forwards requests for incoming shreds to the `Transmitter`, who validates that the requesting node has enough "social credit" to make the request.

4. `Assembler`: This process assembles block shreds into complete blocks.
- Receives: 
  - *Shred Note*: Block shreds from the `Receiver`
- Sends:
  - *Shred Note*: If a received shred is new, sends it to the `Transmitter` to be rebroadcast to the network.
  - *New Block*: Sends assembled complete blocks to the `Ledger` for further processing.

5. `TxPool`: Multiple `TxPool`s, one for each core, are created. Incoming transactions are validated, and a priority queue based on Proof-of-Work difficulty is maintained.
- Receives:
  - *Transaction*: Incoming transactions from the `Receiver`
  - *New Leader Slot*: Triggers the `TxPool` to drain all its transactions and spawn a new `TxExecutor`.

6. `TxExecutor`: Created to execute a list of transactions. The `TxExecutor` takes each `Transaction`, converts it into a `Task`, processes it with the `Bank`, and finally sends the completed `Vec<Transaction>` and `Vec<Task>` to the `Ledger` for inclusion into the block.

7. `OpenPool`: Similar to the `TxPool`, active only during leader mode and dedicated to handling `Open` blocks.
- Receives:
  - *Open*: Incoming open requests from the `Receiver` 
- Sends:
  - *Open List*: When creating a new block, sends the prioritized `Open` blocks to the `Ledger` for further validation and inclusion.

8. `VotePool`: Active only during leader mode, this process maintains a priority queue of votes for the current block.
- Receives:
  - *Vote*: Incoming votes from the `Receiver`
- Sends:
  - *Vote List*: When creating a new block, sends the prioritized votes to the `Ledger` for further validation and inclusion.

9. `Ledger`: Maintains the current state, as well as all pending and finalized blocks, and has the authority to process and finalize transactions. 
- Receives:
  - *Transaction List*: Validated transactions from the `TxPool` in leader mode
  - *Open List*: Validated open requests from the `OpenPool` in leader mode
  - *Vote List*: Validated votes from the `VotePool` in leader mode

10. `Rpc`: Handles RPC requests and responses sent over TCP.
- Sends:
  - *Rpc Request*: Sends incoming RPC requests over the network to the relevant process
- Receives:
  - *Rpc Response*: Incoming RPC responses from the relevant process, which it forwards over the network

## Ledger architecture
The `Ledger` itself depends on several other processes:

1. `Directory`: This process implements a persistent mapping from public keys to account indices.
- Receives:
  - *Batched retrieve request*: Retrieve the indices (or lack thereof) of a list of accounts, given their public keys.
  - *Batched try-insert request* Try to insert a list of (public key, index) key-value pairs into the mapping, failing if they already exist.
- Sends:
  - *Batched retrieve response*: A list of results of the batched retrieve request operation.
  - *Batched try-insert response*: A list of results of the batched try-insert request operation.

2. `Bank`: This object maintains a persistent list of all accounts. It supports the following operations:
- *Push*: Push an empty account with the given public key and representative to the end of the list, then return its index.
- *Pop*: Remove the last account from the end of the list.
- *Queue transfer*: Queue a transfer from one account to another within a given batch: ensure that the transfer can be performed, and modify the batch of the account to prohibit any other transfers from that account from going through.
- *Finish transfer*: Finish a previously queued transfer by moving the funds from the source to the destination.
- *Revert transfer*: Revert a previously finished transfer by moving the funds from the destination to the source.
- *Finalize transfer*: Finalize a previously finished transfer by modifying the finalized balances of both accounts, and updating the weights of the representatives.
- *Finalize change representative*: Finalize a change representative operation by modifying the representative of the account, and updating the weights of the representatives.

3. `Chain`: This process maintains the directed tree that represents all unfinalized blocks, and the list of immutable finalized blocks. The `Chain` keeps the `Bank` up-to-date with the latest finalized blocks, as well as the tip of the longest chain.
- Receives:
  - *Finalize hash*: A hash is requested to be finalized.
  - *Add block*: A block is added to the tree.
- Sends:
  - *Process block*: Instructs a block to be processed by the `Bank`.
  - *Revert block*: Instructs a block to be reverted by the `Bank`.
  - *Finalize block*: Instructs a block to be finalized by the `Bank`.

## Serialization
Starlight uses `bincode` for data serialization / deserialization over the network.

## Data lifecycle
One of the principal concerns of any network-enabled application is how the various data structures should be serialized and deserialized over the network. Ideally, this should involve as little copies as possible.

From the leader's end, the lifecycle of a transaction is roughly as follows:
1) Received: The transaction is received over UDP into the socket receive buffer.
2) Allocated: A new buffer is allocated to contain both the transaction and its hash.
3) Verified: The transaction is hashed and its signature verified. The hash is copied into the transaction's buffer.
4) In pool: The transaction floats in the mempool.
5) Processed: The transaction is queued and processed by the State.
6) Copied: The transaction is copied into a block.
There are two copies: once into the new buffer, and again into the transaction's block.

The lifecycle of a shred is similar:
1) Received: The shred is received over UDP into the socket receive buffer.
2) Allocated: A new buffer is allocated to contain the shred.
3) Integrated: The shred is copied into an incomplete block.
4) Completed: The incomplete block is complete; it is casted into a block and sent to the State.