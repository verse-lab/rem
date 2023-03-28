#!/usr/bin/env python
from dotenv import load_dotenv
import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns
import json
import sys, os, subprocess
from argparse import ArgumentParser
from glob import glob

load_dotenv()

PAPER_FIG_DIR = os.environ.get('PAPER_FIG_DIR')
CAPSTONE_FIG_DIR = os.environ.get('CAPSTONE_FIG_DIR')

sns.set(rc={'figure.figsize':(15, 6)})
sns.set_style('white')

def copy_to_paper(fig_path):
    try:
        ds = [PAPER_FIG_DIR, CAPSTONE_FIG_DIR]
        for d in ds:
            argv = ['cp', fig_path, d]
            p = subprocess.run(argv)
            if p.returncode != 0:
                print('cp to paper dir did not succeed')
    except:
        print('errored cp-ing')


def overall(df):
    fig_path = 'figs/src-size-duration.pdf'
    fig, ax = plt.subplots(figsize=(8,5))
    plt.title('Extraction duration against source file size', fontsize=15, fontweight="bold")
    line = sns.scatterplot(x='SRC_SIZE',
                           y='TOTAL_DURATION_S',
                           size='NUM_INPUTS',
                           sizes=(20,100),
                           hue='NUM_INPUTS',
                           hue_norm=(0,7),
                           marker='o',
                           data=df)
    ax.set_xlabel('Source file size (lines)', fontweight='bold')
    ax.set_ylabel('Duration (s)', fontweight='bold')
    plt.legend(loc='center left', title='Extracted inputs size', bbox_to_anchor=(1,0.5))
    plt.tight_layout()
    plt.savefig(fig_path)
    copy_to_paper(fig_path)

### Relate to cargo cycles
def cargo_cycle_plot(df):
    df_with_cycles = df[df.CARGO_CYCLES > 0]
    df_with_cycles['CARGO_CYCLES'] = df_with_cycles.CARGO_CYCLES.astype(str)
    in_k = lambda i: (i//1000) * 1000 + 1000
    df_with_cycles['PROJECT_SIZE_IN_K'] = df_with_cycles.PROJECT_SIZE.apply(in_k)
    fig_path = 'figs/cargo-cycle-duration.pdf'
    fig, ax = plt.subplots(figsize=(8,5))
    plt.title('Extraction duration against repair cycles', fontsize=15, fontweight="bold")
    line = sns.scatterplot(x='CARGO_CYCLES',
                           y='TOTAL_DURATION_S',
                           marker='o',
                           size='PROJECT_SIZE_IN_K',
                           sizes=(30, 300),
                           legend="full",
                           hue='PROJECT_SIZE_IN_K',
                           data=df_with_cycles)
    boxplt = sns.boxplot(x='CARGO_CYCLES',
                         y='TOTAL_DURATION_S',
                         palette='muted',
                         data=df_with_cycles)
    sns.despine(top=True, left=True, bottom=False)
    ax.set_xlabel('Cargo repair cycle count', fontweight='bold')
    ax.set_ylabel('Duration (s)', fontweight='bold')
    legend_txt = lambda i: f'< {i}'
    handles, labels = ax.get_legend_handles_labels()
    print([h.get_sizes() for h in handles], labels)
    labels = [legend_txt(int(i)) for i in labels]
    plt.legend(handles, labels, bbox_to_anchor=(1.05, 1), loc=2, borderaxespad=0., title='Project Size')
    plt.tight_layout()
    plt.savefig(fig_path)
    copy_to_paper(fig_path)

def features_table(df, name, headers, renames, landscape=False, show=False):
    def get_unique_features(df):
        features = df.FEATURES.unique()
        feat_cols = {}
        for f in features:
            feats = json.loads(f)
            for f in feats:
                feat_cols[f] = f[0].upper() + ' '.join(f[1:].split('_'))
        return feat_cols

    def better_example_name(branch):
        x = branch.rstrip('-expr-active')
        rename = {'ext-com': 'Developer extraction', 'ext': 'Arbitrary extraction', 'inline-ext': 'Inline and extract'}
        for r in rename:
            if x.startswith(r):
                n = x.lstrip(r)
                return f"{rename[r]} {n}"

    def make_latex_table(df, name):
        replace_txt = r'{{{REPLACE_ME}}}'
        fmt = lambda x, y: x.replace(replace_txt, str(y).replace('_', '\\_'))
        alignment = 'r' * (df.shape[1] - 1)
        preamble = r'''\begin{table}[]
\resizebox{\columnwidth}{!}{%
\begin{tabular}{l{{{REPLACE_ME}}}}
\hline'''
        if landscape:
            preamble = r'\begin{landscape}' + '\n' + preamble
        preamble = fmt(preamble, alignment)
        example = df.columns[0]
        header = fmt(r'\textit{\textbf{{{{REPLACE_ME}}}}}', example)
        next_headers_template = r'& \multicolumn{1}{l}{\textit{\textbf{{{{REPLACE_ME}}}}}}'
        for h in df.columns[1:]:
            header += fmt(next_headers_template, h)
        header += r'\\ \hline' + '\n'
        footer = r''' \hline
\end{tabular}%
}
\caption{\tool efficiency on {{{REPLACE_ME}}} project}
\label{table:eff{{{REPLACE_ME}}}}
\end{table}'''
        if landscape:
            footer += '\n' + r'\end{landscape}'
        footer = fmt(footer, name)
        body = ''
        row_template = r' & \textit{{{{REPLACE_ME}}}}'
        for (_,row) in df.iterrows():
            body += row[example].strip('\n')
            for r in df.columns[1:]:
                h = row[r]
                if h == '':
                    body += ' &'
                elif str(h).startswith('\\'):
                    body += f' & {h}'
                else:
                    body += fmt(row_template, str(h).strip('\n'))
            body += r' \\' + '\n'
        body = body.rstrip('\n')
        latex = preamble + '\n' + header + body + footer
        return latex

    def sort_feat_col(x):
        if 'non local' in x.lower():
            return '1' + x
        elif 'borrow' in x.lower():
            return '2' + x
        elif 'lifetime' in x.lower():
            return '3' + x
        return 0
    df['BRANCH'] = df.BRANCH.apply(better_example_name)

    feat_cols = get_unique_features(df)
    for col in feat_cols.keys():
        df[feat_cols[col]] = df.FEATURES_JSON.apply(lambda feats: '\cmark' if col in feats else '')
    sel = [h for h in headers]
    sel.extend(sorted(feat_cols.values(), key=sort_feat_col))
    sel.extend(['TOTAL_DURATION_S'])
    out = df[sel]
    default_renames = {'BRANCH': 'Examples', 'PROJECT_SIZE' : 'Module Size(LOC)', 'TOTAL_DURATION_S': 'Extraction duration(s)', 'CARGO_CYCLES': 'Repair count'}
    default_renames.update(renames)
    out = out.rename(columns=default_renames)
    out.to_csv(f'tables/{name}StatsTbl.csv', index=False, encoding='utf-8')
    latex = make_latex_table(out, name)
    with open(f'tables/{name}StatsTbl.tex', 'w') as f:
        f.write(latex)
        f.flush()
    copy_to_paper(f'tables/{name}StatsTbl.tex')
    return out


def features_table_by_project(df, show=False):
    projects = df.PROJECT.unique()
    projects_with_features = {}
    sel = ['BRANCH', 'PROJECT_SIZE', 'CARGO_CYCLES']

    for project in projects:
        project_df = df[df.PROJECT == project]
        project_df = project_df[project_df.FEATURES_JSON.apply(lambda l: len(l) > 0)]
        if len(project_df) == 0:
            continue
        renames = {'BRANCH': f'{project} examples', 'PROJECT_SIZE' : 'Module Size(LOC)', 'CARGO_CYCLES': 'Repair count'}
        projects_with_features[project] = features_table(project_df, project, sel, renames)
    if show:
        for p in projects_with_features:
            print(p)
            print(projects_with_features[p].head())
            print('\n\n')
            print(latex)
            print('\n\n')

def appendix_table_all_experiment(df, show=False):
    def make_latex_table(df):
        replace_txt = r'{{{REPLACE_ME}}}'
        fmt = lambda x, y: x.replace(replace_txt, str(y).replace('_', '\\_'))
        alignment = 'r' * (df.shape[1] - 1)
        preamble = r'''\begin{landscape}
\begin{table}[]
\resizebox{\columnwidth}{!}{%
\begin{tabular}{l{{{REPLACE_ME}}}}
\hline'''
        preamble = fmt(preamble, alignment)
        example = df.columns[0]
        header = fmt(r'\textit{\textbf{{{{REPLACE_ME}}}}}', example)
        next_headers_template = r'& \multicolumn{1}{l}{\textit{\textbf{{{{REPLACE_ME}}}}}}'
        for h in df.columns[1:]:
            header += fmt(next_headers_template, str(h).strip('\n'))
        header += r'\\ \hline' + '\n'
        footer = r''' \hline
\end{tabular}%
}
\caption{\tool overall experiment result}
\label{table:overallExprResult}
\end{table}
\end{landscape}'''
        body = ''
        row_template = r' & \textit{{{{REPLACE_ME}}}}'
        for (_,row) in df.iterrows():
            body += row[example].strip('\n')
            for r in df.columns[1:]:
                h = row[r]
                if h == '':
                    body += ' &'
                elif str(h).startswith('\\'):
                    body += f' & {h}'
                else:
                    body += fmt(row_template, str(h).strip('\n'))
            body += r' \\' + '\n'
        body = body.rstrip('\n')
        latex = preamble + '\n' + header + body + footer
        return latex

    sel = ['PROJECT', 'BRANCH', 'FIX_NLCF_DURATION_MS', 'FIX_BORROW_DURATION_MS', 'FIX_LIFETIME_CARGO_MS', 'CARGO_CYCLES', 'TOTAL_DURATION_MS',	'COMMIT_URL', 'SUCCESS','FAILED_AT', 'FEATURES']
    out = df[sel]
    latex = make_latex_table(out)
    tbl_path = 'tables/overallExperimentTbl.tex'
    with open(tbl_path, 'w') as f:
        f.write(latex)
        f.flush()
    copy_to_paper(tbl_path)

def inner_handler(csv_path, show=False):
    csv_name = csv_path.split('/')[-1]
    df = pd.read_csv(csv_path)
    df['TOTAL_DURATION_S'] = df.TOTAL_DURATION_S.apply(lambda x: round(x, 3))
    df['FEATURES_JSON'] = df.FEATURES.apply(json.loads)
    overall(df)
    # cargo_cycle_plot(df)
    features_table_by_project(df, show)
    sel = ['PROJECT', 'BRANCH', 'PROJECT_SIZE', 'SRC_SIZE', 'CALLER_SIZE', 'NUM_INPUTS', 'CARGO_CYCLES']
    renames = {'PROJECT': 'Project', 'BRANCH': 'Example', 'PROJECT_SIZE' : 'Module Size(LOC)', 'SRC_SIZE': 'Source file size(LOC)', 'CALLER_SIZE': 'Caller size(LOC)', 'NUM_INPUTS':'Callee input count','CARGO_CYCLES': 'Repair count'}
    features_table(df, "overall", sel, renames, landscape=True, show=show)

    appendix_table_all_experiment(df, show)
    if show:
        plt.show()


def main():
    parser = ArgumentParser()
    parser.add_argument("-c", "--csv_path", action="store")
    parser.add_argument("-v", "--verbose", action="store_true")

    args = parser.parse_args()

    if args.csv_path is None:
        csvs = glob('../results/result_*')
        csvs = sorted(csvs, key=(lambda x: int(x.lstrip('../results/result_').rstrip('.csv'))))
        args.csv_path = csvs[-1]
        print(f'no csv path passed, using: {args.csv_path} ...')
    inner_handler(args.csv_path, args.verbose)


if __name__ == '__main__':
    main()
