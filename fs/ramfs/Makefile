# SPDX-License-Identifier: GPL-2.0-only
#
# Makefile for the linux ramfs routines.
#

obj-y += ramfs.o

# b/c obj-y, etc expects binary file (found this extra-y in kernel docs on makefiles)
extra-y += inode_rs.h

file-mmu-y := file-nommu.o
file-mmu-$(CONFIG_MMU) := file_mmu_rs.o

ramfs-objs += inode_rs.o inode.o $(file-mmu-y)
