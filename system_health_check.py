#!/usr/bin/env python3
"""
System Health Check Tool
ตรวจสอบสถานะ CPU, RAM, Disk ของเครื่อง
"""

import os
import platform
import subprocess

def get_cpu_info():
    """แสดงข้อมูล CPU"""
    print("\n" + "="*60)
    print("CPU INFORMATION")
    print("="*60)
    
    # แสดงจำนวน CPU cores
    try:
        with open('/proc/cpuinfo') as f:
            cpu_info = f.read()
            if 'processor' in cpu_info:
                cores = cpu_info.count('processor')
                print(f"Number of CPU cores: {cores}")
    except FileNotFoundError:
        pass  # ไม่พบ /proc/cpuinfo (อาจเป็น Windows)
    
    # ใช้คำสั่ง wmic สำหรับ Windows
    try:
        result = subprocess.run(['wmic', 'cpu', 'get', 'Name,NumberOfCores,LoadPercentage'], 
                              capture_output=True, text=True, timeout=5)
        if result.returncode == 0:
            lines = result.stdout.strip().split('\n')
            print(result.stdout)
    except (subprocess.TimeoutExpired, FileNotFoundError, PermissionError):
        print("Could not retrieve CPU information")

def get_memory_info():
    """แสดงข้อมูล RAM"""
    print("\n" + "="*60)
    print("MEMORY (RAM) INFORMATION")
    print("="*60)
    
    try:
        # ใช้คำสั่ง wmic สำหรับ Windows
        result = subprocess.run(['wmic', 'OS', 'get', 'TotalVisibleMemorySize,FreePhysicalMemory'], 
                              capture_output=True, text=True, timeout=5)
        if result.returncode == 0:
            lines = result.stdout.strip().split('\n')
            for line in lines[1:]:  # เอา header ออก
                parts = line.strip().split()
                if len(parts) >= 2:
                    total_mb = int(parts[0])
                    free_mb = int(parts[1])
                    total_gb = total_mb / (1024 * 1024)
                    free_gb = free_mb / (1024 * 1024)
                    used_gb = total_gb - free_gb
                    usage_percent = (used_gb / total_gb) * 100 if total_gb > 0 else 0
                    print(f"Total RAM: {total_gb:.2f} GB")
                    print(f"Free RAM: {free_gb:.2f} GB")
                    print(f"Used RAM: {used_gb:.2f} GB ({usage_percent:.1f}%)")
    except (subprocess.TimeoutExpired, FileNotFoundError, PermissionError):
        print("Could not retrieve memory information")

def get_disk_info():
    """แสดงข้อมูล Disk"""
    print("\n" + "="*60)
    print("DISK INFORMATION")
    print("="*60)
    
    try:
        # ใช้คำสั่ง wmic สำหรับ Windows
        result = subprocess.run(['wmic', 'logicaldisk', 'get', 'DeviceID,Label,Size,Freespace,FileSystem'], 
                              capture_output=True, text=True, timeout=5)
        if result.returncode == 0:
            lines = result.stdout.strip().split('\n')
            print(f"{'Drive':<10} {'Size':<15} {'Free':<15} {'File System'}")
            print("-"*60)
            for line in lines[1:]:  # เอา header ออก
                parts = line.strip().split()
                if len(parts) >= 4:
                    drive = parts[0] if parts[0] else 'N/A'
                    size_mb = int(parts[1]) if len(parts) > 1 else 0
                    free_mb = int(parts[2]) if len(parts) > 2 else 0
                    fs = parts[3] if len(parts) > 3 else 'N/A'
                    size_gb = size_mb / (1024 * 1024)
                    free_gb = free_mb / (1024 * 1024)
                    print(f"{drive:<10} {size_gb:>10.2f} GB  {free_gb:>10.2f} GB  {fs}")
    except (subprocess.TimeoutExpired, FileNotFoundError, PermissionError):
        print("Could not retrieve disk information")

def get_system_info():
    """แสดงข้อมูลระบบ"""
    print("\n" + "="*60)
    print("SYSTEM INFORMATION")
    print("="*60)
    
    print(f"Operating System: {platform.system()}")
    print(f"Platform: {platform.platform()}")
    
    # แสดงชื่อเครื่อง
    try:
        hostname = platform.node()
        print(f"Hostname: {hostname}")
    except:
        pass

def main():
    """ฟังก์ชันหลัก"""
    print("\n" + "#"*60)
    print("#" + " SYSTEM HEALTH CHECK TOOL" + "#"*60)
    print("#" + "="*60)
    
    get_system_info()
    get_cpu_info()
    get_memory_info()
    get_disk_info()
    
    print("\n" + "#"*60)
    print("#" + " END OF REPORT" + "#"*60)
    print("#" + "="*60)
    print("\n" + "✅ Health Check Complete!\n")

if __name__ == "__main__":
    main()
