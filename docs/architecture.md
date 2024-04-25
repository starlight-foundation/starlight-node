# Node Architecture 

## Processes
The Starlight node architecture consists of several components, called processes, that work together to enable the functioning of the Starlight blockchain network. Each process can be referenced by a `Handle`, and has a `Mailbox` to receive messages. For efficient message passing, Tokio channels are used behind the scenes. The different processes are:

1. `Scheduler`: This process manages the timing of operations in the Starlight network by controlling when the node switches between different modes based on a schedule. Starlight divides time into slots, and each slot has a designated leader node responsible for creating new blocks. The `Scheduler`'s main job is to track this leader schedule and tell the node when to enter and exit leader mode. When in leader mode, the node creates new blocks.
- Sends:
    - *Start of Leader Mode*: Sent at the start of the slot before the node's first leader slot, telling the node to start accepting transactions from the network. 
    - *End of Leader Mode*: Sent at the end of the node's last leader slot, telling the node to stop accepting transactions.
    - *New Leader Slot*: Sent at the start of each of the node's leader slots, triggering the node to process queued transactions, create a new block, and send it out to the network.

2. `Transmitter`: This process handles the send half of our UDP socket. It maintains the telemetry protocol state, and broadcasts to other nodes.
- Receives:
    - *Telemetry Note*: When the `Receiver` encounters a telemetry message over the wire, it will send it to the `Transmitter` for processing.
    - *Shred Note*: When the `Assembler` needs a shred to be broadcasted to other nodes, or when the `State` needs a newly minted shred to be sent throughout the network, they will send this message, and the `Transmitter` will send the note over the network to a subset of its peers.

3. `Receiver`: This process handles the receive half of the UDP socket, and accepts incoming messages from the network. 
- Sends:
  - *Transaction*: Forwards incoming transactions to the `TxPool` for further processing.
  - *Open*: Forwards incoming open requests to the `OpenPool` for further processing.
  - *Shred Note*: Forwards incoming block shreds to the `Assembler` for assembly.
  - *Telemetry Note*: Forwards incoming telemetry messages to the `Transmitter` for processing

4. `Assembler`: This process assembles block shreds into complete blocks.
- Receives: 
  - *Shred Note*: Block shreds from the `Receiver`
- Sends:
  - *Shred Note*: If a received shred is new, sends it to the `Transmitter` to be rebroadcast to the network.
  - *New Block*: Sends assembled complete blocks to the `State` for further processing.

5. `TxPool`: Active only during leader mode, this process performs initial validation on incoming transactions and maintains a priority queue based on Proof-of-Work difficulty. 
- Receives:
  - *Transaction*: Incoming transactions from the `Receiver`
- Sends: 
  - *Transaction List*: When creating a new block, sends the prioritized transactions to the `State` for further validation and inclusion.

6. `OpenPool`: Similar to the `TxPool`, active only during leader mode and dedicated to handling `Open` blocks.
- Receives:
  - *Open*: Incoming open requests from the `Receiver` 
- Sends:
  - *Open List*: When creating a new block, sends the prioritized `Open` blocks to the `State` for further validation and inclusion.

7. `State`: Maintains the current state, as well as all pending and finalized blocks, and has the authority to process and finalize transactions. 
- Receives:
  - *Transaction List*: Validated transactions from the `TxPool` in leader mode
  - *Open List*: Validated open requests from the `OpenPool` in leader mode

## Serialization
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