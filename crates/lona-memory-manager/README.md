# lona-memory-manager

Lona Memory Manager - seL4 root task for resource management.

This crate is the trusted computing base (TCB) for Lona. It:
- Manages physical memory and capabilities
- Creates and terminates realms
- Handles IPC requests from the Lona VM
- Maps device memory for drivers

## Architecture

The Memory Manager is a minimal, auditable root task that runs with full
seL4 capabilities. It creates the init realm and starts the Lona VM,
then enters an event loop to handle fault IPC and resource requests.

## License

GPL-3.0-or-later
