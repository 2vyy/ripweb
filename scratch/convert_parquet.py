import pandas as pd
import os

def convert_file(filename):
    if not os.path.exists(filename):
        print(f"Error: {filename} not found.")
        return

    out_file = filename.replace(".parquet", ".jsonl")
    print(f"Converting {filename} -> {out_file}...")
    
    try:
        df = pd.read_parquet(filename)
        
        # Extract source_url from question (format is: [Question]\n\nReference URLs:\n- [URL])
        def split_question_url(q):
            parts = q.split("\n\nReference URLs:\n- ")
            if len(parts) > 1:
                return parts[0].strip(), parts[1].strip()
            return q, None

        df[['question', 'source_url']] = df['question'].apply(lambda x: pd.Series(split_question_url(x)))

        # Ensure we are saving in a format eval.rs understands (JSON lines)
        df.to_json(out_file, orient='records', lines=True)
        print(f"Successfully converted {len(df)} rows.")
    except Exception as e:
        print(f"Failed to convert {filename}: {e}")

if __name__ == "__main__":
    convert_file("seal_ref.parquet")
    convert_file("webwalkerqa_ref.parquet")
