#!/usr/bin/env python3
"""
Analyze benchmark results and generate comprehensive report.
Handles high-performance NVMe RAID arrays (>15 GB/s).
"""

import glob
import os
import sys
import numpy as np
from scipy import stats
from pathlib import Path
import json

def load_test_results(test_dir):
    """Load all measurements for a single test suite."""
    results = {
        'elapsed': [],
        'throughput': [],
        'filecount': [],
        'bytes': []
    }
    
    # Load measurements (skip first run - warm-up)
    for pattern in ['elapsed', 'throughput', 'filecount', 'bytes']:
        files = sorted(glob.glob(f"{test_dir}/*_{pattern}.txt"))
        for f in files[1:]:  # Skip run 1
            try:
                with open(f) as fp:
                    value = fp.read().strip()
                    # Handle throughput with units
                    if pattern == 'throughput':
                        value = float(value.replace(' GB/s', ''))
                    else:
                        value = float(value)
                    results[pattern].append(value)
            except (ValueError, FileNotFoundError):
                continue
    
    return results

def calculate_statistics(values):
    """Calculate comprehensive statistics."""
    if not values:
        return {}
    
    return {
        'mean': np.mean(values),
        'median': np.median(values),
        'std': np.std(values),
        'min': np.min(values),
        'max': np.max(values),
        'cv': np.std(values) / np.mean(values) if np.mean(values) > 0 else 0,  # Coefficient of variation
    }

def compare_tests(rsync_results, arsync_results, metric='elapsed'):
    """Statistical comparison between rsync and arsync."""
    rsync_vals = rsync_results[metric]
    arsync_vals = arsync_results[metric]
    
    if not rsync_vals or not arsync_vals:
        return {}
    
    # T-test for statistical significance
    t_stat, p_value = stats.ttest_ind(rsync_vals, arsync_vals)
    
    # Effect size (Cohen's d)
    pooled_std = np.sqrt((np.var(rsync_vals) + np.var(arsync_vals)) / 2)
    if pooled_std > 0:
        cohens_d = (np.mean(rsync_vals) - np.mean(arsync_vals)) / pooled_std
    else:
        cohens_d = 0
    
    # Speedup
    if metric == 'elapsed':
        speedup = np.mean(rsync_vals) / np.mean(arsync_vals) if np.mean(arsync_vals) > 0 else 0
    elif metric == 'throughput':
        speedup = np.mean(arsync_vals) / np.mean(rsync_vals) if np.mean(rsync_vals) > 0 else 0
    else:
        speedup = 0
    
    return {
        'speedup': speedup,
        'p_value': p_value,
        'cohens_d': cohens_d,
        'significant': p_value < 0.05,
        'rsync_mean': np.mean(rsync_vals),
        'arsync_mean': np.mean(arsync_vals),
    }

def generate_report(results_dir):
    """Generate comprehensive benchmark report."""
    results_dir = Path(results_dir)
    
    # Find all test directories
    test_dirs = [d for d in results_dir.iterdir() if d.is_dir()]
    
    # Group by scenario
    scenarios = {}
    for test_dir in test_dirs:
        test_name = test_dir.name
        
        # Extract test type
        if 'rsync' in test_name:
            tool = 'rsync'
        elif 'arsync' in test_name:
            tool = 'arsync'
        else:
            continue
        
        # Extract scenario
        scenario = test_name.split('_', 2)[2] if len(test_name.split('_')) > 2 else test_name
        
        if scenario not in scenarios:
            scenarios[scenario] = {}
        
        scenarios[scenario][tool] = load_test_results(test_dir)
    
    # Generate report
    report = []
    report.append("=" * 80)
    report.append("ARSYNC vs RSYNC - HIGH-PERFORMANCE NVME RAID BENCHMARK RESULTS")
    report.append("=" * 80)
    report.append("")
    
    # Summary table
    report.append("## SUMMARY TABLE")
    report.append("")
    report.append("| Scenario | rsync (s) | arsync (s) | Speedup | p-value | Significant |")
    report.append("|----------|-----------|------------|---------|---------|-------------|")
    
    summary_data = []
    
    for scenario in sorted(scenarios.keys()):
        if 'rsync' not in scenarios[scenario] or 'arsync' not in scenarios[scenario]:
            continue
        
        comparison = compare_tests(
            scenarios[scenario]['rsync'],
            scenarios[scenario]['arsync'],
            metric='elapsed'
        )
        
        if comparison:
            sig_mark = "✓" if comparison['significant'] else "-"
            report.append(
                f"| {scenario:30s} | "
                f"{comparison['rsync_mean']:9.3f} | "
                f"{comparison['arsync_mean']:10.3f} | "
                f"{comparison['speedup']:7.2f}x | "
                f"{comparison['p_value']:7.4f} | "
                f"{sig_mark:11s} |"
            )
            
            summary_data.append({
                'scenario': scenario,
                'rsync_mean': comparison['rsync_mean'],
                'arsync_mean': comparison['arsync_mean'],
                'speedup': comparison['speedup'],
                'p_value': comparison['p_value'],
            })
    
    report.append("")
    report.append("")
    
    # Detailed analysis per scenario
    report.append("## DETAILED ANALYSIS")
    report.append("")
    
    for scenario in sorted(scenarios.keys()):
        if 'rsync' not in scenarios[scenario] or 'arsync' not in scenarios[scenario]:
            continue
        
        report.append(f"### {scenario}")
        report.append("")
        
        rsync_stats = calculate_statistics(scenarios[scenario]['rsync']['elapsed'])
        arsync_stats = calculate_statistics(scenarios[scenario]['arsync']['elapsed'])
        comparison = compare_tests(
            scenarios[scenario]['rsync'],
            scenarios[scenario]['arsync'],
            metric='elapsed'
        )
        
        report.append("**Timing (seconds):**")
        report.append("")
        report.append("| Metric | rsync | arsync |")
        report.append("|--------|-------|--------|")
        report.append(f"| Mean   | {rsync_stats['mean']:.3f} | {arsync_stats['mean']:.3f} |")
        report.append(f"| Median | {rsync_stats['median']:.3f} | {arsync_stats['median']:.3f} |")
        report.append(f"| Std Dev| {rsync_stats['std']:.3f} | {arsync_stats['std']:.3f} |")
        report.append(f"| CV     | {rsync_stats['cv']:.3f} | {arsync_stats['cv']:.3f} |")
        report.append("")
        
        # Throughput analysis
        if scenarios[scenario]['rsync']['throughput'] and scenarios[scenario]['arsync']['throughput']:
            rsync_tp = calculate_statistics(scenarios[scenario]['rsync']['throughput'])
            arsync_tp = calculate_statistics(scenarios[scenario]['arsync']['throughput'])
            
            report.append("**Throughput (GB/s):**")
            report.append("")
            report.append("| Metric | rsync | arsync |")
            report.append("|--------|-------|--------|")
            report.append(f"| Mean   | {rsync_tp['mean']:.2f} | {arsync_tp['mean']:.2f} |")
            report.append(f"| Median | {rsync_tp['median']:.2f} | {arsync_tp['median']:.2f} |")
            report.append("")
        
        report.append(f"**Statistical Analysis:**")
        report.append(f"- Speedup: {comparison['speedup']:.2f}x")
        report.append(f"- p-value: {comparison['p_value']:.4f}")
        report.append(f"- Cohen's d: {comparison['cohens_d']:.2f}")
        report.append(f"- Significant: {'Yes' if comparison['significant'] else 'No'}")
        report.append("")
        report.append("")
    
    # README.md template
    report.append("=" * 80)
    report.append("## README.md TEMPLATE")
    report.append("=" * 80)
    report.append("")
    report.append("```markdown")
    report.append("## Performance Benchmarks")
    report.append("")
    report.append("Benchmarks on Ubuntu 22.04, Linux Kernel 5.15+, 16-core system, NVMe RAID array (>15 GB/s capable):")
    report.append("")
    report.append("| Workload | rsync | arsync | Speedup |")
    report.append("|----------|-------|--------|---------|")
    
    # Find key scenarios for README
    key_scenarios = {
        '100gb': 'Single 100 GB file',
        '10k_small': '10,000 × 10 KB files',
        'deep_d10': 'Deep directory tree',
        'photo_library': 'Mixed workload',
    }
    
    for key, label in key_scenarios.items():
        for scenario, data in summary_data:
            if key in scenario:
                report.append(
                    f"| {label} | {data['rsync_mean']:.2f}s ({data['rsync_mean']:.2f} GB/s) | "
                    f"{data['arsync_mean']:.2f}s ({data['arsync_mean']:.2f} GB/s) | "
                    f"{data['speedup']:.2f}x |"
                )
                break
    
    report.append("```")
    report.append("")
    
    # Save report
    report_text = "\n".join(report)
    output_file = results_dir / "final_report.txt"
    with open(output_file, 'w') as f:
        f.write(report_text)
    
    print(report_text)
    print(f"\nReport saved to: {output_file}")
    
    # Save JSON for programmatic access
    json_file = results_dir / "results.json"
    with open(json_file, 'w') as f:
        json.dump({
            'scenarios': {k: {
                'rsync': {m: [float(v) for v in vs] for m, vs in v['rsync'].items()},
                'arsync': {m: [float(v) for v in vs] for m, vs in v['arsync'].items()}
            } for k, v in scenarios.items() if 'rsync' in v and 'arsync' in v},
            'summary': summary_data
        }, f, indent=2)
    
    print(f"JSON data saved to: {json_file}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python3 analyze_results.py <results_directory>")
        sys.exit(1)
    
    results_dir = sys.argv[1]
    
    if not os.path.isdir(results_dir):
        print(f"Error: {results_dir} is not a directory")
        sys.exit(1)
    
    generate_report(results_dir)

