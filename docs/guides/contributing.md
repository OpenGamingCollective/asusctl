# Contributing

> A guide on how to contribute to the project

## The Spirit

The project is run in a collaborative spirit, where we share each other's knowledge helping one another. It is therefore very important to contribute to the project.

Each contribution is important and helps the project reach its target audience in a meaningful manner, making a difference in other people's Linux experience.

Every contribution counts. If you asked for help, someone else will eventually ask for help solving the same problem. Why not write the solution down? A newcomer who just installed Linux knows the struggle better than a developer who set up their laptop years ago.

## Help-providing contributions

This is the type of contribution that keeps the community alive: answering people's questions when they join Discord with a problem and helping them provide information that will be useful to solve the issue quickly. It is a big part of the community.

## Technical Contributions

Technical contributions are very welcome, and you can choose how much time and effort to dedicate.

Technical contributions include code, documentation, guides, and small tutorials.

Technical contributions take the form of modifications to repositories in the [asus-linux](https://gitlab.com/asus-linux/) GitLab organization.

### Preparations

First, install the needed tools:

```bash
sudo pacman -S code git
```

You will need a GitLab account. You can create one for free or use one of the many supported third-party accounts, such as Google or GitHub.

The next step is to create an SSH key and add it to your account. Do not let the [documentation](https://docs.gitlab.com/user/ssh/) scare you: it is a simple matter and takes a few seconds.

Create the SSH key if you do not have one already:

```bash
ssh-keygen -t ed25519 -C "gitlab"
# Follow the guided procedure.

eval $(ssh-agent -s)
ssh-add "$HOME/.ssh/id_ed25519"

cat "$HOME/.ssh/id_ed25519.pub"

# Copy the output; you will need it later.
```

In your browser, go to [GitLab](https://gitlab.com) and click your profile picture, then **SSH Keys** on the left.

Use the **Add new key** button and paste the content you copied before.

### Forking the repository

To contribute, you will need to send a merge request containing your modifications. Find the project you want to contribute to and fork it.

For example, to add a small guide to the website, find the [website project](https://gitlab.com/asus-linux/website) and fork it using the button in the upper left. This adds a copy of the website to your account once you provide the final confirmation.

### Cloning the code

At this point, you will be redirected to your own copy of the website. To make modifications, download the code: click the colored **Code** button and copy the link under **Clone with SSH**.

```bash
# Replace $URL with the copied URL.
git clone "$URL"
```

This creates a copy of the project on your disk. Enter it and launch an editor, for example Visual Studio Code:

```bash
cd website
code .
```

### Editing the code

You can now use your editor to modify the website. When you are done, use the Git integration to send your contribution back to GitLab: add the files you modified, write a meaningful commit message, and commit the changes.

To send a merge request, return to your fork in GitLab and use the prompt that appears above the list of files.

Someone from the project will get back to you when they have time.
