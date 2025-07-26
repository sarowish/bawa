# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Select the newly created games and profiles in the list.
- Place entries relative to the currently selected entry when performing a move operation.
- Add a command for loading random save files

### Fixed

- Handle file move events where the destination or the source is outside the watched directories.
- Update the savefile path of the selected game instead of the active game when setting paths.
