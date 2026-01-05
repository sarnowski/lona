/* SPDX-License-Identifier: GPL-3.0-or-later */
/* Linker script for Lona VM - places binary at SHARED_CODE_BASE */

/* SHARED_CODE_BASE from lona-abi/src/layout.rs */
SHARED_CODE_BASE = 0x0000000100000000;

ENTRY(_start)

PHDRS {
    text   PT_LOAD FLAGS(5); /* PF_R | PF_X */
    rodata PT_LOAD FLAGS(4); /* PF_R */
    data   PT_LOAD FLAGS(6); /* PF_R | PF_W */
    tls    PT_TLS  FLAGS(4); /* PF_R - TLS template */
}

SECTIONS {
    . = SHARED_CODE_BASE;

    /* Code section - RX */
    .text : ALIGN(4K) {
        *(.text.unlikely .text.unlikely.*)
        *(.text .text.*)
        *(.gnu.linkonce.t.*)
    } :text

    /* Read-only data - RO */
    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
        *(.gnu.linkonce.r.*)
    } :rodata

    /* Thread-local storage template - used by sel4 crate for IPC buffer */
    /* TLS sections are in both data (for loading) and tls (for template metadata) */
    .tdata : ALIGN(4K) {
        __tdata_start = .;
        *(.tdata .tdata.*)
        __tdata_end = .;
    } :data :tls

    .tbss : {
        __tbss_start = .;
        *(.tbss .tbss.*)
        __tbss_end = .;
    } :data :tls

    /* Global Offset Table - placed explicitly after TLS to avoid TLS pollution */
    .got : ALIGN(8) {
        *(.got)
        *(.got.plt)
    } :data

    /* Read-write data - RW */
    .data : ALIGN(4K) {
        *(.data .data.*)
        *(.gnu.linkonce.d.*)
    } :data

    .bss : ALIGN(4K) {
        __bss_start = .;
        *(.bss .bss.*)
        *(.gnu.linkonce.b.*)
        *(COMMON)
        __bss_end = .;
    } :data

    /* Discard debug sections to reduce size */
    /DISCARD/ : {
        *(.comment)
        *(.note.*)
    }
}
