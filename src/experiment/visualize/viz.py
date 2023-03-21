#!/usr/bin/env python
from dotenv import load_dotenv
import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns
import sys, os, subprocess

load_dotenv()

PAPER_FIG_DIR = os.environ.get('PAPER_FIG_DIR')

csv_path = sys.argv[1]
csv_name = csv_path.split('/')[-1]

sns.set(rc={'figure.figsize':(15, 6)})
sns.set_style('white')

df = pd.read_csv(csv_path)

def copy_to_paper(fig_path):
    try:
        argv = ['cp', fig_path, f'{PAPER_FIG_DIR}/']
        p = subprocess.run(argv)
        if p.returncode != 0:
            print('cp to paper dir did not succeed')
    except:
        print('errored cp-ing')

### Initial overall view
def overall():
    fig_path = 'figs/src-size-duration.pdf'
    fig, ax = plt.subplots(figsize=(8,5))
    plt.title('Extraction duration against source file size', fontsize=15, fontweight="bold")
    line = sns.lineplot(x='SRC_SIZE',
                        y='TOTAL_DURATION_S',
                        marker='o',
                        data=df)
    ax.set_xlabel('Source file size (lines)', fontweight='bold')
    ax.set_ylabel('Duration (s)', fontweight='bold')
    plt.tight_layout()
    plt.savefig(fig_path)
    copy_to_paper(fig_path)

### Relate to cargo cycles
def cargo_cycle_plot():
    df_with_cycles = df[df.CARGO_CYCLES > 0]
    df_with_cycles['CARGO_CYCLES'] = df_with_cycles.CARGO_CYCLES.astype(str)
    in_k = lambda i: (i//1000) * 1000
    df_with_cycles['PROJECT_SIZE_IN_K'] = df_with_cycles.PROJECT_SIZE.apply(in_k)
    fig_path = 'figs/cargo-cycle-duration.pdf'
    fig, ax = plt.subplots(figsize=(8,5))
    plt.title('Extraction duration against repair cycles', fontsize=15, fontweight="bold")
    line = sns.scatterplot(x='CARGO_CYCLES',
                           y='TOTAL_DURATION_S',
                           marker='o',
                           size='PROJECT_SIZE',
                           hue='PROJECT_SIZE',
                           sizes=(100, 300),
                           palette='deep',
                           data=df_with_cycles)
    boxplt = sns.boxplot(x='CARGO_CYCLES',
                         y='TOTAL_DURATION_S',
                         palette='muted',
                         data=df_with_cycles)
    sns.despine(top=True, left=True, bottom=False)
    ax.set_xlabel('Cargo repair cycle count', fontweight='bold')
    ax.set_ylabel('Duration (s)', fontweight='bold')
    def legend_txt(i):
        if i == 0:
            return '< 1000'
        else:
            return f'> {i}'
    ax.legend(labels=dict([ (legend_txt(i), i) for i in df_with_cycles.PROJECT_SIZE_IN_K]), title='Project Size')
    plt.tight_layout()
    plt.savefig(fig_path)
    copy_to_paper(fig_path)

overall()
cargo_cycle_plot()
plt.show()