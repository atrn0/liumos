#pragma once

#include "asm.h"
#include "kernel_virtual_heap_allocator.h"
#include "paging.h"

class PersistentMemoryManager;

class SegmentMapping {
 public:
  void Set(uint64_t vaddr, uint64_t paddr, uint64_t map_size) {
    vaddr_ = vaddr;
    paddr_ = paddr;
    map_size_ = map_size;
    CLFlush(this);
  }
  uint64_t GetPhysAddr() { return paddr_; }
  void SetPhysAddr(uint64_t paddr) {
    paddr_ = paddr;
    CLFlush(&paddr);
  }
  uint64_t GetVirtAddr() { return vaddr_; }
  uint64_t GetMapSize() { return map_size_; }
  uint64_t GetVirtEndAddr() { return vaddr_ + map_size_; }
  void Clear() {
    paddr_ = 0;
    vaddr_ = 0;
    map_size_ = 0;
    CLFlush(this);
  }
  void AllocSegmentFromPersistentMemory(PersistentMemoryManager& pmem);
  void Print();
  void CopyDataFrom(SegmentMapping& from, uint64_t& stat_copied_bytes);
  template <class TAllocator>
  void Map(TAllocator& allocator,
           IA_PML4& page_root,
           uint64_t page_attr,
           bool shoud_clflush) {
    if (!GetPhysAddr())
      return;  // To avoid mapping null segment
    CreatePageMapping(allocator, page_root, GetVirtAddr(), GetPhysAddr(),
                      GetMapSize(), kPageAttrPresent | page_attr,
                      shoud_clflush);
  }

  void Flush(IA_PML4& pml4, uint64_t& num_of_clflush_issued);

 private:
  uint64_t vaddr_;
  uint64_t paddr_;
  uint64_t map_size_;
};

struct ProcessMappingInfo {
  SegmentMapping code;
  SegmentMapping data;
  SegmentMapping stack;
  SegmentMapping heap;
  void Print();
  void Clear() {
    code.Clear();
    data.Clear();
    stack.Clear();
    heap.Clear();
  }
  void Flush(IA_PML4& pml4, uint64_t& num_of_clflush_issued) {
    code.Flush(pml4, num_of_clflush_issued);
    data.Flush(pml4, num_of_clflush_issued);
    heap.Flush(pml4, num_of_clflush_issued);
  }
};

class ExecutionContext {
 public:
  CPUContext& GetCPUContext() { return cpu_context_; }
  ProcessMappingInfo& GetProcessMappingInfo() { return map_info_; };
  void PushDataToStack(const void* data, size_t byte_size);
  void AlignStack(int align);
  uint64_t GetRSP() { return cpu_context_.int_ctx.rsp; }
  uint64_t GetKernelRSP() { return kernel_rsp_; }
  void SetKernelRSP(uint64_t kernel_rsp) { kernel_rsp_ = kernel_rsp; }
  void ExpandHeap(int64_t diff);
  uint64_t GetHeapEndVirtAddr() {
    return heap_used_size_ + map_info_.heap.GetVirtAddr();
  }
  void SetCR3(IA_PML4& cr3) {
    cpu_context_.cr3 = reinterpret_cast<uint64_t>(&cr3);
  }
  IA_PML4& GetCR3() { return *reinterpret_cast<IA_PML4*>(cpu_context_.cr3); }
  void SetRegisters(void (*rip)(),
                    uint16_t cs,
                    void* rsp,
                    uint16_t ss,
                    uint64_t cr3,
                    uint64_t rflags,
                    uint64_t kernel_rsp) {
    cpu_context_.int_ctx.rip = reinterpret_cast<uint64_t>(rip);
    cpu_context_.int_ctx.cs = cs;
    cpu_context_.int_ctx.rsp = reinterpret_cast<uint64_t>(rsp);
    cpu_context_.int_ctx.ss = ss;
    cpu_context_.int_ctx.rflags = rflags | 2;
    // 10.2.3 MXCSR Control and Status Register
    // Mask all FPU exceptions
    /*
    for(int i = 0; i < 512; i++){
      cpu_context_.fpu_context.data[i] = 0;
    }
    cpu_context_.fpu_context.data[24] = 0x80;
    cpu_context_.fpu_context.data[25] = 0x1F;
    cpu_context_.fpu_context.data[26] = 0x00;
    cpu_context_.fpu_context.data[27] = 0x00;
    */
    cpu_context_.cr3 = cr3;
    kernel_rsp_ = kernel_rsp;
    heap_used_size_ = 0;
  }
  void Flush(IA_PML4& pml4, uint64_t& stat);
  void CopyContextFrom(ExecutionContext& from, uint64_t& stat_copied_bytes) {
    uint64_t cr3 = cpu_context_.cr3;
    cpu_context_ = from.cpu_context_;
    cpu_context_.cr3 = cr3;

    map_info_.data.CopyDataFrom(from.map_info_.data, stat_copied_bytes);
    map_info_.stack.CopyDataFrom(from.map_info_.stack, stat_copied_bytes);
  }

 private:
  CPUContext cpu_context_;
  ProcessMappingInfo map_info_;
  uint64_t kernel_rsp_;
  uint64_t heap_used_size_;
};

class PersistentProcessInfo {
 public:
  bool IsValid() { return signature_ == kSignature; }
  void Print();
  void Init() {
    valid_ctx_idx_ = kNumOfExecutionContext;
    CLFlush(&valid_ctx_idx_);
    signature_ = kSignature;
    CLFlush(&signature_);
  }
  ExecutionContext& GetContext(int idx) {
    assert(0 <= idx && idx < kNumOfExecutionContext);
    return ctx_[idx];
  }
  ExecutionContext& GetValidContext() {
    assert(0 <= valid_ctx_idx_ && valid_ctx_idx_ < kNumOfExecutionContext);
    return ctx_[valid_ctx_idx_];
  }
  ExecutionContext& GetWorkingContext() {
    assert(0 <= valid_ctx_idx_ && valid_ctx_idx_ < kNumOfExecutionContext);
    return ctx_[1 - valid_ctx_idx_];
  }
  void SetValidContextIndex(int idx) {
    valid_ctx_idx_ = idx;
    CLFlush(&valid_ctx_idx_);
  }
  static constexpr uint64_t kSignature = 0x4F50534F6D75696CULL;
  static constexpr int kNumOfExecutionContext = 2;
  void SwitchContext(uint64_t& stat_copied_bytes,
                     uint64_t& stat_num_of_clflush);

 private:
  ExecutionContext ctx_[kNumOfExecutionContext];
  int valid_ctx_idx_;
  uint64_t signature_;
};
