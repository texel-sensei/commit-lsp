# Multistage remote lookup

To support different remotes (e.g. Github, Gitlab, AzureDevOps) an adapter is used.

However each adapter has different needs to be initialized (e.g. credentials).

To be flexible, the initialization of an adapter is performed in multiple steps.

## 1. Get URL and type of remote

Required:
   - What kind of issue tracker do we have?
   - What is the URL where we can find issues?

1. Check repository config for type/url overrides
    1. If none, grab remote url (default url of `origin`)
    2. Check user config for type overrides based on url
    3. If none, guess remote kind from url

## 2. Create Adapter Builder

Construct a builder that can query the remaining required information.
The builder gets access to the user and repo configs and can grab what it needs.
E.g. a Jira builder would need to grab the issue prefix.
