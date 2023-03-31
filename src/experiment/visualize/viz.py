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

def features_table(df, name, renames, longTable=False, landscape=False, show=False, resize_to_width=False):
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
        #rename = {'ext-com': 'Developer extraction', 'ext': 'Arbitrary extraction', 'inline-ext': 'Inline and extract'}
        rename = {'ext-com': 'DE', 'ext': 'AE', 'inline-ext': 'IE'}
        for r in rename:
            if x.startswith(r):
                # n = x.lstrip(r)
                return f"{rename[r]}"

    def make_latex_table(df, name, features, resize_to_width):
        project_inner_merged = {}
        projects = df.Project.unique()
        df['ID'] = [(i + 1) for i in range(df.shape[0])]

        for project in projects:
            project_df = df[df.Project == project]
            project_inner_merged[project] = {'row': project_df.shape[0]}

        merged = {'Project': {'row':2}, 'ET': {'row':2}, 'Feature': {'col':len(features)}}
        replace_txt = r'[[[REPLACE_ME]]]'
        fmt = lambda x, y: x.replace(replace_txt, str(y).replace('_', '\\_'))
        fmt1 = lambda x, y: x.replace(replace_txt, str(y).replace('_', '\\_'), 1)
        features_starts_at = 4
        alignment = ('r' * ((features_starts_at - 1) + len(features) - 1)).replace('r','r|',2)
        preamble = r'\begin{table}[]'+'\n'+r'\centering'
        if resize_to_width:
            preamble += '\n' + r'\resizebox{\columnwidth}{!}{%'
        preamble += '\n' + r'''\begin{tabular}{l|[[[REPLACE_ME]]]}
\hline'''
        if landscape:
            preamble = r'\begin{landscape}' + '\n' + preamble
        preamble = fmt(preamble, alignment)
        header = r'\multirow{2}{*}{\textit{\textbf{\#}}}'
        next_headers_template = r'& [[[REPLACE_ME]]]{\textit{\textbf{[[[REPLACE_ME]]]}}}'
        for h in merged:
            tmp = next_headers_template
            if 'col' in merged[h]:
                if h == 'Feature':
                    tmp = fmt1(tmp, r'\multicolumn{'+str(merged[h]['col'])+r'}{c}') #c| if line
                else:
                    tmp = fmt1(tmp, r'\multicolumn{'+str(merged[h]['col'])+r'}{l}')
            elif 'row' in merged[h]:
                tmp = fmt1(tmp, r'\multirow{'+str(merged[h]['row'])+r'}{*}')
            header += fmt1(tmp, h)
        # header += r'& \multirow{2}{*}{\textit{\textbf{Time(s)}}}'
        header += r'\\ \cline{'+str(features_starts_at)+'-' + str(features_starts_at+len(features) - 1) + '}\n'
        features_header = ' &' * (features_starts_at - 2)
        features_abbr = lambda i: ''.join([j[0] for j in i.split(' ')]).upper()
        for f in features:
            features_header += fmt(r' & \textit{\textbf{[[[REPLACE_ME]]]}}', features_abbr(f))
        # features_header += '& '
        header += features_header
        header += r'\\ \hline' + '\n'
        footer = r''' \hline
\end{tabular}%'''
        if resize_to_width:
            footer += '\n' + r'}'
        footer += '\n' + r'''\caption{\tool . }
\label{table:eff[[[REPLACE_ME]]]}
\end{table}'''
        if landscape:
            footer += '\n' + r'\end{landscape}'
        footer = fmt(footer, name)
        current_project = ''
        body = ''
        row_template = r' & \textit{[[[REPLACE_ME]]]}'
        project_template = r' & \multirow{[[[REPLACE_ME]]]}{*}{\textit{[[[REPLACE_ME]]]}}'
        for (_,row) in df.iterrows():
            if current_project == row['Project']:
                body += str(row['ID'])
                body += ' &'
            else:
                if current_project != '':
                    body = body[:-1] + r' \hline' + '\n'
                body += str(row['ID'])
                current_project = row['Project']
                body += fmt1(fmt1(project_template, project_inner_merged[row['Project']]['row']), current_project)
            body += fmt(row_template, row['ET'])
            for r in features:
                h = row[r]
                if h == '':
                    body += ' &'
                elif str(h).startswith('\\'):
                    body += f' & {h}'
                else:
                    body += fmt(row_template, str(h).strip('\n'))
            # body += fmt(row_template, row['t(s)'])
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
    features = sorted(feat_cols.values(), key=sort_feat_col)
    default_renames = {'PROJECT':'Project', 'BRANCH': 'ET'}
    default_renames.update(renames)
    out = df.rename(columns=default_renames)
    out.to_csv(f'tables/{name}StatsTbl.csv', index=False, encoding='utf-8')
    latex = make_latex_table(out, name, features, resize_to_width)
    with open(f'tables/{name}StatsTbl.tex', 'w') as f:
        f.write(latex)
        f.flush()
    copy_to_paper(f'tables/{name}StatsTbl.tex')
    return out

def inner_handler(csv_path, show=False):
    csv_name = csv_path.split('/')[-1]
    df = pd.read_csv(csv_path)
    df['TOTAL_DURATION_S'] = df.TOTAL_DURATION_S.apply(lambda x: round(x, 3))
    df['FEATURES_JSON'] = df.FEATURES.apply(json.loads)
    overall(df)
    # cargo_cycle_plot(df)
    #features_table_by_project(df, show)
    renames = {'PROJECT': 'Project', 'BRANCH': 'ET', 'TOTAL_DURATION_S': 't(s)'}
    features_table(df, "overall", renames, landscape=False, show=show)
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
