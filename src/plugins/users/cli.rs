use clap::Args;
use frunk::{HCons, hlist::HList};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    app::MountedApp,
    command::{CommandCapability, CommandRegistrar, RunCommand},
    plugins::users::{
        UsersTag, auth, seed,
        entities::user::{self, Entity as UserEntity},
        error::UsersError,
        state::UsersState,
    },
    tag::Tagged,
    traits::get::GetByTag,
};

// Tag for [`CreateSuperuserCommand`].
pub struct CreateSuperuserCommandTag;

// Tag for [`ChangePasswordCommand`].
pub struct ChangePasswordCommandTag;

// Tag for [`RevalidateUsersCommand`].
pub struct RevalidateUsersCommandTag;

// Create an admin superuser.
#[derive(Clone, Copy, Debug, Default)]
pub struct CreateSuperuserCommand;

// Args for [`CreateSuperuserCommand`].
#[derive(Args, Debug, Clone)]
pub struct CreateSuperuserArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub phone: String,
    #[arg(long)]
    pub password: String,
}

#[async_trait::async_trait]
impl<M, UsersIdx> RunCommand<M, UsersIdx> for CreateSuperuserCommand
where
    M: GetByTag<UsersTag, UsersIdx, Value = UsersState> + Sync + Send + 'static,
    UsersIdx: Send + Sync + 'static,
{
    type Args = CreateSuperuserArgs;
    const NAME: &'static str = "createsuperuser";
    const ABOUT: &'static str = "Create an admin superuser";

    async fn run(args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
        let state = app.get_capability_output::<UsersTag, UsersIdx>();
        createsuperuser(state, args.name, args.email, args.phone, &args.password).await?;
        println!("superuser created");
        Ok(())
    }
}

// Change a user's password.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChangePasswordCommand;

// Args for [`ChangePasswordCommand`].
#[derive(Args, Debug, Clone)]
pub struct ChangePasswordArgs {
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub password: String,
}

#[async_trait::async_trait]
impl<M, UsersIdx> RunCommand<M, UsersIdx> for ChangePasswordCommand
where
    M: GetByTag<UsersTag, UsersIdx, Value = UsersState> + Sync + Send + 'static,
    UsersIdx: Send + Sync + 'static,
{
    type Args = ChangePasswordArgs;
    const NAME: &'static str = "changepassword";
    const ABOUT: &'static str = "Change a user's password";

    async fn run(args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
        let state = app.get_capability_output::<UsersTag, UsersIdx>().clone();
        changepassword(&state, &args.email, &args.password).await?;
        println!("password updated");
        Ok(())
    }
}

// Normalize stored user emails and phone numbers.
#[derive(Clone, Copy, Debug, Default)]
pub struct RevalidateUsersCommand;

// Args for [`RevalidateUsersCommand`].
#[derive(Args, Debug, Clone, Default)]
pub struct RevalidateUsersArgs {}

#[async_trait::async_trait]
impl<M, UsersIdx> RunCommand<M, UsersIdx> for RevalidateUsersCommand
where
    M: GetByTag<UsersTag, UsersIdx, Value = UsersState> + Sync + Send + 'static,
    UsersIdx: Send + Sync + 'static,
{
    type Args = RevalidateUsersArgs;
    const NAME: &'static str = "revalidate-users";
    const ABOUT: &'static str = "Normalize stored user emails and phone numbers";

    async fn run(_args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
        let state = app.get_capability_output::<UsersTag, UsersIdx>().clone();
        let n = revalidate_users(&state).await?;
        println!("revalidated {n} users");
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl<C> CommandRegistrar<C> for Hook
where
    C: HList + Clone,
{
    type Output = UsersCommands<C>;

    fn register_commands(self, cap: CommandCapability<C>) -> CommandCapability<Self::Output> {
        cap.prepend::<CreateSuperuserCommandTag, _>(CreateSuperuserCommand)
            .prepend::<ChangePasswordCommandTag, _>(ChangePasswordCommand)
            .prepend::<RevalidateUsersCommandTag, _>(RevalidateUsersCommand)
    }
}

// Commands registered by [`Hook`] for [`UsersTag`].
pub type UsersCommands<C> = HCons<
    Tagged<RevalidateUsersCommandTag, RevalidateUsersCommand>,
    HCons<
        Tagged<ChangePasswordCommandTag, ChangePasswordCommand>,
        HCons<Tagged<CreateSuperuserCommandTag, CreateSuperuserCommand>, C>,
    >,
>;

pub async fn createsuperuser(
    state: &UsersState,
    name: String,
    email: String,
    phone: String,
    password: &str,
) -> Result<(), UsersError> {
    let role = seed::ensure_unassigned_role(&state.db).await?;
    auth::create_user(
        &state.db,
        auth::CreateUser {
            name,
            email,
            phone,
            plain_password: password.to_owned(),
            role_id: role.id,
            is_superuser: true,
            timezone: None,
        },
    )
    .await?;
    Ok(())
}

pub async fn changepassword(
    state: &UsersState,
    email: &str,
    password: &str,
) -> Result<(), UsersError> {
    let user = UserEntity::find()
        .filter(user::Column::Email.eq(email))
        .one(&state.db)
        .await?
        .ok_or(UsersError::NotFound)?;
    let am: user::ActiveModel = user.into();
    auth::set_password(&state.db, am, password).await?;
    Ok(())
}

pub async fn revalidate_users(state: &UsersState) -> Result<usize, UsersError> {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set};

    let users = UserEntity::find()
        .filter(user::Column::DeletedAt.is_null())
        .all(&state.db)
        .await?;
    let mut updated = 0usize;
    for user in users {
        let email = user.email.trim().to_lowercase();

        let mut phone = user.phone.clone();
        if let Ok(parsed) = phonenumber::parse(Some(phonenumber::country::IN), &phone)
            && parsed.is_valid()
        {
            phone = parsed.format().mode(phonenumber::Mode::E164).to_string();
        }

        if email != user.email || phone != user.phone {
            let mut am: user::ActiveModel = user.into();
            am.email = Set(email);
            am.phone = Set(phone);
            am.updated_at = Set(Some(Utc::now()));
            am.update(&state.db).await?;
            updated += 1;
        }
    }
    Ok(updated)
}
