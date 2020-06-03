#pragma once

#include <optional>

#include "generic.h"
#include "kernel.h"
#include "liumos.h"
#include "pci.h"

namespace Virtio {
class Net {
 public:
  struct PacketBufHeader {
    // virtio: 5.1.6 Device Operation
    uint8_t flags;
    uint8_t gso_type;
    uint16_t header_length;
    uint16_t gso_size;
    uint16_t csum_start;
    uint16_t csum_offset;
    //
    static constexpr uint8_t kFlagNeedsChecksum = 1;
    static constexpr uint8_t kGSOTypeNone = 0;
  };
  packed_struct ARPPacket {
    uint8_t dst[6];
    uint8_t src[6];
    uint8_t eth_type[2];
    uint8_t hw_type[2];
    uint8_t proto_type[2];
    uint8_t hw_addr_len;
    uint8_t proto_addr_len;
    uint8_t op[2];
    uint8_t sender_eth_addr[6];
    uint8_t sender_proto_addr[4];
    uint8_t target_eth_addr[6];
    uint8_t target_proto_addr[4];

    void SetupRequest(const uint8_t(&target_ip)[4], const uint8_t(&src_ip)[4],
                      const uint8_t(&src_mac)[6]) {
      for (int i = 0; i < 6; i++) {
        dst[i] = 0xff;
        src[i] = src_mac[i];
        sender_eth_addr[i] = src_mac[i];
        target_eth_addr[i] = 0x00;
      }
      for (int i = 0; i < 4; i++) {
        sender_proto_addr[i] = src_ip[i];
        target_proto_addr[i] = target_ip[i];
      }
      eth_type[0] = 0x08;
      eth_type[1] = 0x06;
      hw_type[0] = 0x00;
      hw_type[1] = 0x01;
      proto_type[0] = 0x08;
      proto_type[1] = 0x00;
      hw_addr_len = 6;
      proto_addr_len = 4;
      op[0] = 0x00;
      op[1] = 0x01;
    }
  };
  packed_struct IPv4UDPPacket {
    // Ethernet
    uint8_t dst[6];
    uint8_t src[6];
    uint8_t eth_type[2];
    // IPv4
    uint8_t version_and_ihl;
    uint8_t dscp_and_ecn;
    uint8_t total_length[2];
    uint16_t ident;
    uint16_t flags;
    uint8_t ttl;
    uint8_t protocol;
    uint16_t checksum;
    uint8_t src_ip[4];
    uint8_t dst_ip[4];
    // UDP
    uint8_t src_port[2];  // optional
    uint8_t dst_port[2];
    uint8_t udp_length[2];
    uint8_t udp_checksum[2];
    uint32_t udp_data;

    void CalcAndSetChecksum() {
      uint16_t* p = reinterpret_cast<uint16_t*>(&this->version_and_ihl);
      uint32_t sum = 0;
      for (int i = 0; i < 5; i++) {
        if (i == 5) {
          // Skip checksum itself
          continue;
        }
        sum += (*p >> 8) | ((*p << 8) & 0xff00);
      }
      while (checksum >> 16) {
        sum = (sum & 0xffff) + (sum >> 16);
      }
      checksum = sum;
    }

    void SetupRequest(const uint8_t(&dst_ip)[4], const uint8_t(&src_ip)[4],
                      const uint8_t(&src_mac)[6], const uint8_t(&next_mac)[6]) {
      for (int i = 0; i < 6; i++) {
        dst[i] = next_mac[i];
        src[i] = src_mac[i];
      }
      for (int i = 0; i < 4; i++) {
        this->src_ip[i] = src_ip[i];
        this->dst_ip[i] = dst_ip[i];
      }
      eth_type[0] = 0x08;
      eth_type[1] = 0x00;
      version_and_ihl = 0x45;  // version 4, header length is 20 bytes
      dscp_and_ecn = 0x00;
      total_length[0] = 0x00;
      total_length[1] = 20;  // overall size of ip packet including ip header
      ident = 0x4242;
      flags = 0x0040;  // don't fragment
      ttl = 32;
      protocol = 17;  // UDP
      CalcAndSetChecksum();
      // UDP
      src_port[0] = 0;
      src_port[1] = 0;
      dst_port[0] = 0;
      dst_port[1] = 80;
      udp_length[0] = 0;
      udp_length[1] = 4;
      udp_checksum[0] = 0;
      udp_checksum[1] = 0;
      udp_data = 0x55AA55AA;
    }
  };
  static_assert(sizeof(IPv4UDPPacket) == 20 + 6 * 2 + 2 + 2 * 4 + 4);
  class Virtqueue {
   public:
    packed_struct Descriptor {
      volatile uint64_t addr;
      volatile uint32_t len;
      volatile uint16_t flags;
      volatile uint16_t next;
    };
    struct UsedRingEntry {
      volatile uint32_t id;   // index of start of used descriptor chain
      volatile uint32_t len;  // in byte
    };
    void Alloc(int queue_size);
    uint64_t GetPhysAddr() { return v2p(reinterpret_cast<uint64_t>(base_)); }
    void SetDescriptor(int idx,
                       void* vaddr,
                       uint32_t len,
                       uint16_t flags,
                       uint16_t next) {
      assert(0 <= idx && idx < queue_size_);
      buf_[idx] = vaddr;
      Descriptor& desc =
          *reinterpret_cast<Descriptor*>(base_ + sizeof(Descriptor) * idx);
      desc.addr = v2p(vaddr);
      desc.len = len;
      desc.flags = flags;
      desc.next = next;
    }
    void* GetDescriptorBuf(int idx) {
      assert(0 <= idx && idx < queue_size_);
      return buf_[idx];
    }
    uint32_t GetDescriptorSize(int idx) {
      assert(0 <= idx && idx < queue_size_);
      Descriptor& desc =
          *reinterpret_cast<Descriptor*>(base_ + sizeof(Descriptor) * idx);
      return desc.len;
    }
    void SetAvailableRingEntry(int idx, uint16_t desc_idx) {
      assert(0 <= idx && idx < queue_size_);
      volatile uint16_t* ring = reinterpret_cast<volatile uint16_t*>(
          base_ + sizeof(Descriptor) * queue_size_ + sizeof(uint16_t) * 2);
      ring[idx] = desc_idx;
    }
    void SetAvailableRingIndex(int idx) {
      volatile uint16_t& pidx = *reinterpret_cast<volatile uint16_t*>(
          base_ + sizeof(Descriptor) * queue_size_ + sizeof(uint16_t));
      pidx = idx;
    }
    uint16_t GetUsedRingIndex() {
      // This function returns the index of used ring
      // which will be written by device on the next data arriving.
      volatile uint16_t& pidx = *reinterpret_cast<volatile uint16_t*>(
          base_ +
          CeilToPageAlignment(sizeof(Descriptor) * queue_size_ +
                              sizeof(uint16_t) * (2 * queue_size_)) +
          sizeof(uint16_t));
      return pidx;
    }
    UsedRingEntry& GetUsedRingEntry(int idx) {
      assert(0 <= idx && idx < queue_size_);
      UsedRingEntry* used_ring = reinterpret_cast<UsedRingEntry*>(
          base_ +
          CeilToPageAlignment(sizeof(Descriptor) * queue_size_ +
                              sizeof(uint16_t) * (2 * queue_size_)) +
          2 * sizeof(uint16_t));
      return used_ring[idx];
    }

   private:
    static constexpr int kMaxQueueSize = 0x100;
    int queue_size_;
    uint8_t* base_;
    void* buf_[kMaxQueueSize];
  };

  void Init();

  static Net& GetInstance() {
    if (!net_) {
      net_ = liumos->kernel_heap_allocator->Alloc<Net>();
      bzero(net_, sizeof(Net));
      new (net_) Net();
    }
    assert(net_);
    return *net_;
  }

 private:
  static constexpr int kNumOfVirtqueues = 3;

  static constexpr int kIndexOfRXVirtqueue = 0;
  static constexpr int kIndexOfTXVirtqueue = 1;

  static Net* net_;
  PCI::DeviceLocation dev_;
  uint8_t mac_addr_[6];
  uint16_t config_io_addr_base_;
  Virtqueue vq_[kNumOfVirtqueues];

  uint8_t ReadConfigReg8(int ofs);
  uint16_t ReadConfigReg16(int ofs);
  uint32_t ReadConfigReg32(int ofs);

  void WriteConfigReg8(int ofs, uint8_t data);
  void WriteConfigReg16(int ofs, uint16_t data);
  void WriteConfigReg32(int ofs, uint32_t data);

  uint8_t ReadDeviceStatus();
  void WriteDeviceStatus(uint8_t);
  void SetFeatures(uint32_t);
};
};  // namespace Virtio
