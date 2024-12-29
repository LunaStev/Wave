import os
import praw

def main():
    reddit = praw.Reddit(
        client_id=os.getenv("REDDIT_CLIENT_ID"),
        client_secret=os.getenv("REDDIT_SECRET"),
        username=os.getenv("REDDIT_USERNAME"),
        password=os.getenv("REDDIT_PASSWORD"),
        user_agent="github-to-reddit/2.0 (by u/Pinggu12222)"
    )

    subreddit_name = os.getenv("REDDIT_SUBREDDIT")
    subreddit = reddit.subreddit(subreddit_name)

    commit_message = os.getenv("GITHUB_HEAD_COMMIT_MESSAGE", "New commit pushed to GitHub")

    subreddit.submit(
        title="New Update in Wave Repository!",
        selftext=f"A new commit has been pushed:\n\n{commit_message}"
    )
    print("Posted to Reddit successfully!")

if __name__ == "__main__":
    main()
