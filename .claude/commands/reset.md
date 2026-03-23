# Reset Workspace

Reset the workspace by checking out the default branch and pulling latest changes.

## Steps

1. Detect the default branch by running `git symbolic-ref refs/remotes/origin/HEAD | sed 's@^refs/remotes/origin/@@'`. If that fails, fall back to checking if `main` or `master` exists.
2. Run `git checkout <default-branch>`.
3. Run `git pull`.
4. Report the current branch and latest commit.
