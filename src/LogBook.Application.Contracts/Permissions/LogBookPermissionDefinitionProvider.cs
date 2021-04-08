using LogBook.Localization;
using Volo.Abp.Authorization.Permissions;
using Volo.Abp.Localization;

namespace LogBook.Permissions
{
    public class LogBookPermissionDefinitionProvider : PermissionDefinitionProvider
    {
        public override void Define(IPermissionDefinitionContext context)
        {
            var myGroup = context.AddGroup(LogBookPermissions.GroupName);

            //Define your own permissions here. Example:
            //myGroup.AddPermission(LogBookPermissions.MyPermission1, L("Permission:MyPermission1"));
        }

        private static LocalizableString L(string name)
        {
            return LocalizableString.Create<LogBookResource>(name);
        }
    }
}
